#[derive(Debug, Clone)]
pub enum Statement {
    // let x = 5;
    VarDeclaration {
        name: String,
        var_type: String,
        initializer: ExpressionNode,
        is_mutable: bool,
    },

    // x = 10;
    Assignment {
        name: String,
        value: ExpressionNode,
    },

    IndexAssignment {
        target: ExpressionNode,
        index: ExpressionNode,
        value: ExpressionNode,
    },

    // if cond { then } else { else }
    If {
        condition: ExpressionNode,
        then_branch: Box<StatementNode>,
        else_branch: Option<Box<StatementNode>>, // Optional 'else'
    },

    // fn name(a, b) { body }
    FunctionDeclaration {
        name: String,
        params: Vec<ExpressionNode>,
        body: Vec<StatementNode>,
    },

    // return 5
    Return(Option<ExpressionNode>),

    // break 5, break
    Break(Option<ExpressionNode>),

    // continue
    Continue,

    // { stmt1; stmt2; }
    Block(Vec<StatementNode>),

    // A standalone expression acting as a statement (e.g., calling print(x);)
    Expression(ExpressionNode),

    Match {
        target: ExpressionNode,
        cases: Vec<MatchCase>,
        default_branch: Option<Box<StatementNode>>,
    },
}

#[derive(Debug, Clone)]
pub struct MatchCase {
    pub pattern: ExpressionNode, // The value to compare against (e.g. "what", 5, true)
    pub body: Box<StatementNode>, // The statement or block code execution path
}

#[derive(Debug, Clone)]
pub enum Expression {
    // A raw value like 5, "hello", or true
    Literal(LiteralValue),

    // Accessing a variable by name (e.g., x)
    Variable {
        name: String,
        is_mutable: bool,
    },

    // Clean property tracking (e.g., standard_list . append)
    Get {
        object: Box<ExpressionNode>,
        name: String,
    },

    Array(Vec<ExpressionNode>),

    // Binary operations (e.g., a + b, x < 10)
    Binary {
        operator: BinaryOperator,
        left: Box<ExpressionNode>,
        right: Box<ExpressionNode>,
    },

    // Unary operations (e.g., -x, !flag)
    Unary {
        operator: UnaryOperator,
        right: Box<ExpressionNode>,
    },

    // Calling a function (e.g., add(1, 2))
    Call {
        callee: Box<ExpressionNode>,
        arguments: Vec<ExpressionNode>,
    },

    Index {
        object: Box<ExpressionNode>,
        index: Box<ExpressionNode>,
    },

    // while cond { body }
    While {
        condition: Box<ExpressionNode>,
        body: Box<StatementNode>,
    },

    For {
        item_name: String,
        iterable: Box<ExpressionNode>,
        body: Box<StatementNode>,
    },
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Int(i32),
    Float(f64),
    String(String),
    Boolean(bool),
    Nil, // Represents null/none
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div, // Math
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessEqual,
    GreaterEqual, // Comparison
    And,
    Or, // Logic
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Minus, // Negation: -x
    Not,   // Logical NOT: !x
}

#[derive(Debug, Clone)]
pub struct ExpressionNode {
    pub span: Span,       // Line and column metadata
    pub kind: Expression, // The actual enum (Literal, Binary, Call, etc.)
}

// 2. The wrapper for all statements
#[derive(Debug, Clone)]
pub struct StatementNode {
    pub span: Span,      // Line and column metadata
    pub kind: Statement, // The actual enum (VarDeclaration, If, While, etc.)
}

#[derive(Debug)]
pub struct Program {
    pub body: Vec<StatementNode>, // A flat list of top-level statements
}

use crate::lexer::Span;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, index: 0 }
    }

    fn error(&self, message: &str, span: Span) -> crate::AppError {
        crate::AppError::SyntaxError {
            line: span.line,
            column: span.column,
            message: message.to_string(),
        }
    }

    fn expect_token(&mut self, expected: &str, message: &str) -> Result<Token, crate::AppError> {
        let span = self.peek_span();
        if self.peek() == Some(expected) {
            Ok(self.advance().unwrap())
        } else {
            Err(self.error(message, span))
        }
    }

    fn advance_or_err(&mut self, message: &str) -> Result<Token, crate::AppError> {
        let span = self.peek_span();
        self.advance().ok_or_else(|| self.error(message, span))
    }

    // look at next token
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.index).map(|t| t.value.as_str())
    }

    // Grab the span of the next token to associate with your AST node
    fn peek_span(&self) -> Span {
        self.tokens
            .get(self.index)
            .map(|t| t.span)
            .unwrap_or_else(|| {
                // Fallback for EOF: use the span of the very last token if available
                self.tokens
                    .last()
                    .map(|t| t.span)
                    .unwrap_or(Span { line: 1, column: 1 })
            })
    }

    // advances forward by one token if it can
    fn advance(&mut self) -> Option<Token> {
        if self.index < self.tokens.len() {
            let token = self.tokens[self.index].clone();
            self.index += 1;
            Some(token)
        } else {
            None
        }
    }

    // checks if next token matches 'expected' &str
    fn match_token(&mut self, expected: &str) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            return true;
        }
        false
    }

    // parses statement nodes until done then returns the program AST
    pub fn parse_program(&mut self) -> Result<Program, crate::AppError> {
        let mut body = Vec::new();
        while self.peek().is_some() {
            body.push(self.parse_statement()?);
        }
        Ok(Program { body })
    }

    pub fn parse_statement(&mut self) -> Result<StatementNode, crate::AppError> {
        if let Some(token) = self.peek() {
            match token {
                "let" => self.parse_var_declaration(),
                "if" => self.parse_if_statement(),
                "match" => self.parse_match_statement(),
                "fn" => self.parse_function_declaration(),
                "return" => self.parse_return_statement(),
                "break" => self.parse_break_statement(),
                "continue" => self.parse_continue_statement(),
                "{" => self.parse_block_statement(),
                _ => self.parse_expression_statement(),
            }
        } else {
            Err(self.error(
                "Unexpected end of input while looking for a statement",
                self.peek_span(),
            ))
        }
    }

    // parses variable declarations ( let variable = value )
    fn parse_var_declaration(&mut self) -> Result<StatementNode, crate::AppError> {
        let start_span: Span = self.peek_span(); // Grab the exact position of "let"
        self.advance(); // consume "let"

        // assumed not mutable by default, is changed to true if 'mut' keyword is present after 'let'
        let mut is_mutable: bool = false;

        if self.peek() == Some("mut") {
            is_mutable = true;
            self.advance();
        }

        // gets variable name
        let name_token: Token = self.advance_or_err("Expected variable name after 'let'")?;
        let name: String = name_token.value;

        // assumes variable type as 'Undeclared' unless typed after ':' ( let variable: type = value)
        let mut var_type: String = "Undeclared".to_string();

        if self.peek() == Some(":") {
            self.advance(); // consume ":"
            let mut type_tokens = Vec::new();

            // Consume tokens until we find the assignment operator '='
            while let Some(tok) = self.peek() {
                if tok == "=" {
                    break;
                }
                type_tokens.push(self.advance().unwrap().value);
            }
            var_type = type_tokens.join(""); // Merges "List", "[", "int", "]" into "List[int]"
        }

        self.expect_token("=", "Expected '=' after variable name")?;
        let initializer = self.parse()?;

        Ok(StatementNode {
            span: start_span, // Assign the dynamic starting position
            kind: Statement::VarDeclaration {
                name,
                var_type,
                initializer,
                is_mutable,
            },
        })
    }

    // Parses: if ( condition ) { body }
    fn parse_if_statement(&mut self) -> Result<StatementNode, crate::AppError> {
        let start_span = self.peek_span();

        self.advance(); // consume "if"

        let condition = self.parse()?;

        let then_branch = Box::new(self.parse_statement()?);
        let mut else_branch = None;
        if self.match_token("else") {
            else_branch = Some(Box::new(self.parse_statement()?));
        }

        Ok(StatementNode {
            span: start_span,
            kind: Statement::If {
                condition,
                then_branch,
                else_branch,
            },
        })
    }

    fn parse_match_statement(&mut self) -> Result<StatementNode, crate::AppError> {
        let start_span = self.peek_span();
        self.advance(); // consume "match"

        let target = self.parse()?; // Parse the evaluation expression target

        self.expect_token("{", "Expected '{{' to open match block")?;

        let mut cases = Vec::new();
        let mut default_branch = None;

        while let Some(tok) = self.peek() {
            if tok == "}" {
                break;
            }

            // Check for the wildcard catch-all pattern "_"
            if self.peek() == Some("_") {
                self.advance(); // consume "_"
                self.expect_token("=>", "Expected '=>' thick arrow after fallback pattern '_'")?;

                default_branch = Some(Box::new(self.parse_statement()?));
                continue;
            }

            // Parse normal value patterns (e.g., "what", 5, or true)
            let pattern = self.parse()?;
            self.expect_token(
                "=>",
                "Expected '=>' thick arrow after match pattern condition",
            )?;

            let body = if self.peek() == Some("{") {
                Box::new(self.parse_block_statement()?)
            } else {
                Box::new(self.parse_statement()?)
            };

            cases.push(MatchCase { pattern, body });
        }

        self.expect_token("}", "Expected '}}' to close match block execution frame")?;

        Ok(StatementNode {
            span: start_span,
            kind: Statement::Match {
                target,
                cases,
                default_branch,
            },
        })
    }

    // Parses: fn my_func ( mut a : Type , b ) { ... }
    fn parse_function_declaration(&mut self) -> Result<StatementNode, crate::AppError> {
        let start_span: Span = self.peek_span();
        self.advance(); // consume "fn"

        let name: String = self
            .advance_or_err("Expected function name after 'fn'")?
            .value;
        self.expect_token("(", "Expected '(' after function name")?;

        let mut params: Vec<ExpressionNode> = Vec::new();

        if self.peek() != Some(")") {
            loop {
                let mut is_mutable: bool = false;

                if self.peek() == Some("mut") {
                    is_mutable = true;
                    self.advance();
                }
                let param_name: String = self.advance_or_err("Expected parameter name")?.value;

                // 3. Skip type annotations if a colon ':' is present
                if self.match_token(":") {
                    // Keep consuming tokens until we see a ',' or ')'
                    while let Some(token) = self.peek() {
                        if token == "," || token == ")" {
                            break;
                        }
                        self.advance(); // consume type tokens like "List[int]"
                    }
                }
                let param: ExpressionNode = ExpressionNode {
                    span: start_span,
                    kind: Expression::Variable {
                        name: param_name,
                        is_mutable,
                    },
                };
                params.push(param);

                // break if there's no comma separating the next parameter
                if !self.match_token(",") {
                    break;
                }
            }
        }

        self.expect_token(")", "Expected ')' after parameter list")?;

        if self.peek() != Some("{") {
            return Err(self.error("Expected '{' to open function body", self.peek_span()));
        }

        let body_stmt = self.parse_statement()?;
        let body = match body_stmt.kind {
            Statement::Block(statements) => statements,
            _ => unreachable!(),
        };

        Ok(StatementNode {
            span: start_span,
            kind: Statement::FunctionDeclaration { name, params, body },
        })
    }

    // Parses: return 5  or  return
    fn parse_return_statement(&mut self) -> Result<StatementNode, crate::AppError> {
        let start_span = self.peek_span();
        self.advance(); // consume "return"

        let mut value = None;
        // Check if the next token belongs to a different statement or block closer
        if let Some(tok) = self.peek() {
            let is_boundary = matches!(
                tok,
                "}" | "return"
                    | "break"
                    | "continue"
                    | "let"
                    | "if"
                    | "while"
                    | "for"
                    | "match"
                    | "fn"
            );
            if !is_boundary {
                value = Some(self.parse()?);
            }
        }

        Ok(StatementNode {
            span: start_span,
            kind: Statement::Return(value),
        })
    }

    fn parse_break_statement(&mut self) -> Result<StatementNode, crate::AppError> {
        let start_span = self.peek_span();
        self.advance(); // consume "break"

        let mut value = None;
        // Check if the next token belongs to a different statement or block closer
        if let Some(tok) = self.peek() {
            let is_boundary = matches!(
                tok,
                "}" | "return"
                    | "break"
                    | "continue"
                    | "let"
                    | "if"
                    | "while"
                    | "for"
                    | "match"
                    | "fn"
            );
            if !is_boundary {
                value = Some(self.parse()?);
            }
        }

        Ok(StatementNode {
            span: start_span,
            kind: Statement::Break(value),
        })
    }

    fn parse_continue_statement(&mut self) -> Result<StatementNode, crate::AppError> {
        let start_span = self.peek_span();
        self.advance(); // consume "continue"

        Ok(StatementNode {
            span: start_span,
            kind: Statement::Continue,
        })
    }

    // Parses: { stmt1 stmt2 }
    fn parse_block_statement(&mut self) -> Result<StatementNode, crate::AppError> {
        let start_span = self.peek_span();
        self.advance(); // consume "{"

        let mut statements = Vec::new();
        while let Some(tok) = self.peek() {
            if tok == "}" {
                break;
            }
            statements.push(self.parse_statement()?);
        }

        self.expect_token("}", "Expected '}' to close block")?;

        Ok(StatementNode {
            span: start_span,
            kind: Statement::Block(statements),
        })
    }

    // Fallback: x = 10  or  print(x)
    fn parse_expression_statement(&mut self) -> Result<StatementNode, crate::AppError> {
        let start_span = self.peek_span();
        let expr = self.parse()?;

        if self.match_token("=") {
            let value = self.parse()?;
            match expr.kind {
                Expression::Variable { name, .. } => {
                    return Ok(StatementNode {
                        span: start_span,
                        kind: Statement::Assignment { name, value },
                    });
                }
                Expression::Index { object, index } => {
                    return Ok(StatementNode {
                        span: start_span,
                        kind: Statement::IndexAssignment {
                            target: *object,
                            index: *index,
                            value,
                        },
                    });
                }
                _ => {
                    return Err(self.error("Invalid assignment target.", start_span));
                }
            }
        }

        Ok(StatementNode {
            span: start_span,
            kind: Statement::Expression(expr),
        })
    }

    // --- Expression Processing Rules ---
    // first
    pub fn parse(&mut self) -> Result<ExpressionNode, crate::AppError> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<ExpressionNode, crate::AppError> {
        let mut left = self.parse_logical_and()?;

        while self.peek() == Some("||") {
            let start_span = left.span;
            self.advance(); // consume "||"
            let right = self.parse_logical_and()?;

            left = ExpressionNode {
                span: start_span,
                kind: Expression::Binary {
                    operator: BinaryOperator::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<ExpressionNode, crate::AppError> {
        let mut left = self.parse_comparison()?;

        while self.peek() == Some("&&") {
            let start_span = left.span;
            self.advance(); // consume "&&"
            let right = self.parse_comparison()?;

            left = ExpressionNode {
                span: start_span,
                kind: Expression::Binary {
                    operator: BinaryOperator::And,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<ExpressionNode, crate::AppError> {
        let mut left = self.parse_additive()?;

        while let Some(token) = self.peek() {
            let operator = match token {
                "==" => BinaryOperator::Equal,
                "!=" => BinaryOperator::NotEqual,
                "<" => BinaryOperator::LessThan,
                ">" => BinaryOperator::GreaterThan,
                "<=" => BinaryOperator::LessEqual,
                ">=" => BinaryOperator::GreaterEqual,
                _ => break,
            };

            let start_span = left.span;
            self.advance(); // Consume the comparison operator token
            let right = self.parse_additive()?;

            left = ExpressionNode {
                span: start_span,
                kind: Expression::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    // Intercepts preceding '-' or '!' symbols before diving into literals
    fn parse_unary(&mut self) -> Result<ExpressionNode, crate::AppError> {
        if let Some(token) = self.peek() {
            let operator = match token {
                "-" => Some(UnaryOperator::Minus),
                "!" => Some(UnaryOperator::Not),
                _ => None,
            };

            if let Some(op) = operator {
                let start_span = self.peek_span();
                self.advance(); // Consume the operator symbol token

                // Recursively call parse_unary to correctly support chained elements like !!true
                let right = self.parse_unary()?;

                return Ok(ExpressionNode {
                    span: start_span,
                    kind: Expression::Unary {
                        operator: op,
                        right: Box::new(right),
                    },
                });
            }
        }
        // Fallback naturally to raw numbers/variables if no prefix matches
        self.parse_primary()
    }

    fn parse_additive(&mut self) -> Result<ExpressionNode, crate::AppError> {
        let mut left = self.parse_multiplicative()?;
        while let Some(token) = self.peek() {
            let operator = match token {
                "+" => BinaryOperator::Add,
                "-" => BinaryOperator::Sub,
                _ => break,
            };

            let start_span = left.span;
            self.advance();
            let right = self.parse_multiplicative()?;
            left = ExpressionNode {
                span: start_span,
                kind: Expression::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<ExpressionNode, crate::AppError> {
        // Route through our call & property loop interceptor
        let mut left = self.parse_call_or_member()?;
        while let Some(token) = self.peek() {
            let operator = match token {
                "*" => BinaryOperator::Mul,
                "/" => BinaryOperator::Div,
                _ => break,
            };
            let start_span = left.span;
            self.advance();
            let right = self.parse_call_or_member()?;
            left = ExpressionNode {
                span: start_span,
                kind: Expression::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_call_or_member(&mut self) -> Result<ExpressionNode, crate::AppError> {
        let mut expr = self.parse_unary()?;

        loop {
            let start_span = expr.span;
            if self.match_token("(") {
                // Parse call arguments
                let mut arguments = Vec::new();
                if self.peek() != Some(")") {
                    loop {
                        arguments.push(self.parse()?);
                        if !self.match_token(",") {
                            break;
                        }
                    }
                }
                self.expect_token(")", "Expected ')' after arguments")?;

                expr = ExpressionNode {
                    span: start_span,
                    kind: Expression::Call {
                        callee: Box::new(expr),
                        arguments,
                    },
                };
            } else if self.match_token(".") {
                // Pure member lookup! No more string concatenation hacks.
                let property = self
                    .advance_or_err("Expected field or method name after '.'")?
                    .value;
                expr = ExpressionNode {
                    span: start_span,
                    kind: Expression::Get {
                        object: Box::new(expr),
                        name: property,
                    },
                };
            } else if self.match_token("[") {
                let index = self.parse()?;
                self.expect_token("]", "Expected ']' after index expression")?;
                expr = ExpressionNode {
                    span: start_span,
                    kind: Expression::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                    },
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<ExpressionNode, crate::AppError> {
        let start_span = self.peek_span(); // Grab the position before consuming tokens

        let mut is_mutable = false;
        if self.peek() == Some("mut") {
            is_mutable = true;
            self.advance();
        }

        let token_obj = self.advance_or_err("Unexpected end of expression")?;
        let token = token_obj.value;

        if token == "(" {
            let sub_expr = self.parse();
            self.expect_token(")", "Expected closing parenthesis ')'")?;
            return sub_expr; // Keep sub-expression's internal span
        }

        if token == "[" {
            if is_mutable {
                return Err(self.error(
                    "Cannot apply 'mut' modifier to an array literal initialization",
                    start_span,
                ));
            }

            let mut elements = Vec::new();
            if self.peek() != Some("]") {
                loop {
                    elements.push(self.parse()?);
                    if !self.match_token(",") {
                        break;
                    }
                }
            }
            self.expect_token("]", "Expected closing bracket ']' after array elements")?;

            return Ok(ExpressionNode {
                span: start_span,
                kind: Expression::Array(elements),
            });
        }

        if let Ok(number) = token.parse::<f64>() {
            let literal = if token.contains('.') {
                LiteralValue::Float(number)
            } else {
                LiteralValue::Int(number as i32)
            };

            return Ok(ExpressionNode {
                span: start_span,
                kind: Expression::Literal(literal),
            });
        }

        // will parse strings as literals ("a", 'a')
        if (token.starts_with('"') && token.ends_with('"'))
            || (token.starts_with('\'') && token.ends_with('\''))
        {
            if is_mutable {
                return Err(self.error(
                    "Cannot apply 'mut' modifier to a string literal",
                    start_span,
                ));
            }

            // Extract the inside text by stripping out the bounding quotes
            let inner_value = token[1..token.len() - 1]
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\r", "\r");

            return Ok(ExpressionNode {
                span: start_span,
                kind: Expression::Literal(LiteralValue::String(inner_value)),
            });
        }

        if token == "while" {
            if is_mutable {
                return Err(self.error("Cannot apply 'mut' modifier to a while loop", start_span));
            }
            let condition = Box::new(self.parse()?);
            let body = Box::new(self.parse_statement()?);
            return Ok(ExpressionNode {
                span: start_span,
                kind: Expression::While { condition, body },
            });
        }

        if token == "for" {
            if is_mutable {
                return Err(self.error("Cannot apply 'mut' modifier to a for loop", start_span));
            }
            let item_name = self
                .advance_or_err("Expected variable name after 'for'")?
                .value;
            self.expect_token(
                "in",
                "Expected 'in' keyword after variable name in for loop",
            )?;
            let iterable = Box::new(self.parse()?);
            let body = Box::new(self.parse_statement()?);
            return Ok(ExpressionNode {
                span: start_span,
                kind: Expression::For {
                    item_name,
                    iterable,
                    body,
                },
            });
        }

        if token == "true" {
            if is_mutable {
                return Err(self.error(
                    "Cannot apply 'mut' modifier to a boolean literal",
                    start_span,
                ));
            }
            return Ok(ExpressionNode {
                span: start_span,
                kind: Expression::Literal(LiteralValue::Boolean(true)),
            });
        }

        if token == "false" {
            if is_mutable {
                return Err(self.error(
                    "Cannot apply 'mut' modifier to a boolean literal",
                    start_span,
                ));
            }
            return Ok(ExpressionNode {
                span: start_span,
                kind: Expression::Literal(LiteralValue::Boolean(false)),
            });
        }

        if token == "nil" || token == "null" {
            if is_mutable {
                return Err(self.error("Cannot apply 'mut' modifier to a nil literal", start_span));
            }
            return Ok(ExpressionNode {
                span: start_span,
                kind: Expression::Literal(LiteralValue::Nil),
            });
        }

        Ok(ExpressionNode {
            span: start_span,
            kind: Expression::Variable {
                name: token,
                is_mutable,
            },
        })
    }
}
