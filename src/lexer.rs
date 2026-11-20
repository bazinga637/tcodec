#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub value: String,
    pub span: Span,
}

pub struct Lexer {
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source_code: String) -> Self {
        Lexer {
            chars: source_code.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
        }
    }

    fn error(&self, message: &str, span: Span) -> crate::AppError {
        crate::AppError::LexicalError {
            line: span.line,
            column: span.column,
            message: message.to_string(),
        }
    }
    // see current token without consuming it
    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }
    // see next token without consuming any tokens
    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }
    // consumes current token
    fn advance(&mut self) -> Option<char> {
        let current_char = self.peek()?;
        self.index += 1;

        if current_char == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(current_char)
    }
    // main lexer logic
    pub fn lex(mut self) -> Result<Vec<Token>, crate::AppError> {
        let symbols = vec![
            '(', ')', '{', '}', '\n', '*', '+', '-', '=', '|', '&', ':', '.', ',', '[', ']',
        ];
        let ignored_chars = [' ', '\t', '\r'];

        let mut tokens: Vec<Token> = vec![];
        let mut current_str = String::new();
        let mut word_start_span = Span { line: 1, column: 1 };

        while let Some(c) = self.peek() {
            // handle comments ('//')
            if c == '/' && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                while let Some(comment_char) = self.peek() {
                    if comment_char == '\n' {
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            // Handle compound assignment operators: +=, -=, *=, /=
            if (c == '+' || c == '-' || c == '*' || c == '/' || c == '%')
                && self.peek_next() == Some('=')
            {
                if !current_str.is_empty() {
                    tokens.push(Token {
                        value: current_str.clone(),
                        span: word_start_span,
                    });
                    current_str.clear();
                }
                let op_span = Span {
                    line: self.line,
                    column: self.column,
                };
                let first = self.advance().unwrap();
                let second = self.advance().unwrap();
                tokens.push(Token {
                    value: format!("{}{}", first, second),
                    span: op_span,
                });
                continue;
            }

            // Handle Independent Slash
            if c == '/' {
                let slash_span = Span {
                    line: self.line,
                    column: self.column,
                };
                self.advance();
                tokens.push(Token {
                    value: "/".to_string(),
                    span: slash_span,
                });
                continue;
            }

            // Handle Ignored Whitespace
            if ignored_chars.contains(&c) {
                if !current_str.is_empty() {
                    tokens.push(Token {
                        value: current_str.clone(),
                        span: word_start_span,
                    });
                    current_str.clear();
                }
                self.advance();
                continue;
            }

            // handle string literals ('...' or "...")
            if c == '"' || c == '\'' {
                if !current_str.is_empty() {
                    tokens.push(Token {
                        value: current_str.clone(),
                        span: word_start_span,
                    });
                    current_str.clear();
                }

                let string_start_span = Span {
                    line: self.line,
                    column: self.column,
                };
                let quote_char = self.advance().unwrap();
                let mut string_literal = String::new();

                string_literal.push(quote_char);

                while let Some(inner_c) = self.peek() {
                    if inner_c == '\\' {
                        string_literal.push(self.advance().unwrap());
                        if let Some(escaped) = self.advance() {
                            string_literal.push(escaped);
                        }
                        continue;
                    }
                    if inner_c == quote_char {
                        break;
                    }
                    string_literal.push(self.advance().unwrap());
                }

                if self.peek() == Some(quote_char) {
                    string_literal.push(self.advance().unwrap());
                } else {
                    return Err(self.error("Unterminated string literal.", string_start_span));
                }

                tokens.push(Token {
                    value: string_literal,
                    span: string_start_span,
                });
                continue;
            }

            // handles comparisons and match statement thick arrows (=>)
            if c == '=' || c == '<' || c == '!' || c == '>' {
                if !current_str.is_empty() {
                    tokens.push(Token {
                        value: current_str.clone(),
                        span: word_start_span,
                    });
                    current_str.clear();
                }

                let comparison_start_span = Span {
                    line: self.line,
                    column: self.column,
                };
                let starting_char = self.advance().unwrap();
                let mut sequence = starting_char.to_string();

                // Check for multi-character sequences like ==, <=, >=, != or =>
                if (starting_char == '=' && self.peek() == Some('>')) || self.peek() == Some('=') {
                    sequence.push(self.advance().unwrap());
                }

                tokens.push(Token {
                    value: sequence,
                    span: comparison_start_span,
                });
                continue;
            }

            // AND and OR boolean operators
            if c == '|' || c == '&' {
                if !current_str.is_empty() {
                    tokens.push(Token {
                        value: current_str.clone(),
                        span: word_start_span,
                    });
                    current_str.clear();
                }
                let andor_start_span = Span {
                    line: self.line,
                    column: self.column,
                };

                let andor_operator = if self.peek_next() == Some(c) {
                    self.advance();
                    self.advance();
                    format!("{}{}", c, c)
                } else {
                    return Err(self.error(
                        &format!("Unexpected character '{}'. Did you mean '{}{}'?", c, c, c),
                        andor_start_span,
                    ));
                };

                tokens.push(Token {
                    value: andor_operator,
                    span: andor_start_span,
                });
                continue;
            }

            // handle symbols
            if symbols.contains(&c) {
                if c == '.'
                    && !current_str.is_empty()
                    && current_str.chars().all(|ch| ch.is_ascii_digit())
                    && self
                        .peek_next()
                        .is_some_and(|next_c| next_c.is_ascii_digit())
                {
                    current_str.push(self.advance().unwrap());
                    continue;
                }

                if !current_str.is_empty() {
                    tokens.push(Token {
                        value: current_str.clone(),
                        span: word_start_span,
                    });
                    current_str.clear();
                }

                let symbol_span = Span {
                    line: self.line,
                    column: self.column,
                };
                let sym = self.advance().unwrap();

                if sym != '\n' {
                    tokens.push(Token {
                        value: sym.to_string(),
                        span: symbol_span,
                    });
                }
                continue;
            }

            // accumulate identifiers, keywords, and literals
            if current_str.is_empty() {
                word_start_span = Span {
                    line: self.line,
                    column: self.column,
                };
            }

            current_str.push(c);
            self.advance();

            // Intercept 'match' to prevent greedy string blending (e.g., matching 'match_variable')
            if current_str == "fn"
                || current_str == "let"
                || current_str == "mut"
                || current_str == "match"
            {
                if let Some(next) = self.peek() {
                    if ignored_chars.contains(&next)
                        || symbols.contains(&next)
                        || next == '/'
                        || next == '='
                        || next == '<'
                        || next == '>'
                        || next == '!'
                    {
                        tokens.push(Token {
                            value: current_str.clone(),
                            span: word_start_span,
                        });
                        current_str.clear();
                    }
                }
            }
        }

        if !current_str.is_empty() {
            tokens.push(Token {
                value: current_str,
                span: word_start_span,
            });
        }
        Ok(tokens)
    }
}
