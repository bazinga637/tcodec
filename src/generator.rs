use crate::parser::{
    BinaryOperator, Expression, ExpressionNode, LiteralValue, Program, Statement, StatementNode,
};
use std::collections::HashMap;
use std::rc::Rc;

use crate::lexer::Span;
use crate::parser::CompoundOperator;
/*
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ownership {
    Owned,
    SharedBorrow,
    MutableBorrow,
}
*/
#[derive(Debug, Clone)]
pub enum OpCode {
    PushInt(i32),
    PushFloat(f64),
    PushString(Rc<String>),
    PushBoolean(bool),
    PushNil,

    // Memory / Variable Commands
    GetLocal(usize), // Read variable value from index slot
    SetLocal(usize), // Pop value and store into index slot

    GetIndex,
    SetIndex,

    GetLocalIndex(usize),
    SetLocalIndex(usize),
    CompoundAssignLocalIndex(usize, CompoundOperator),

    // Evaluation Commands
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    CompoundAssignLocal(usize, CompoundOperator),
    CompoundAssignIndex(CompoundOperator),
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    Negate, // -x
    Not,    // !x
    Pop,    // Cleans the stack after a standalone expression statement

    // Structures
    BuildArray(usize), // Pop N items to construct an array literal
    CallMethod { name: Rc<String>, arg_count: usize },
    CallFunction { name: Rc<String>, arg_count: usize },
    // Jumps (The usize is the instruction pointer index to jump to)
    Jump(usize),
    JumpIfFalse(usize),

    Return,

    LineUpdate(Span),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowKind {
    Shared, // Equivalent to &
    Mut,    // Equivalent to &mut
}

#[derive(Debug, Clone)]
pub struct Loan {
    pub provider_name: String, // The source variable being borrowed (e.g., "numbers")
    pub kind: BorrowKind,      // Kind of borrow (shared or mutable)
    pub borrow_scope_depth: usize, // Which scope level created this lease?
}

#[derive(Debug, Clone)]
pub struct VariableMetadata {
    pub slot: usize,
    pub is_mutable: bool,
    // Track who THIS variable is currently borrowing from
    //pub borrowing_from: Option<Loan>,
    // Track who is actively borrowing FROM this variable
    pub active_loans: Vec<Loan>,
}

pub struct Scope {
    // Map names to an expanded tracking structure instead of a raw usize slot
    pub variables: HashMap<String, VariableMetadata>,
    pub depth_start_index: usize,
}

pub struct LoopContext {
    pub continue_jumps: Vec<usize>,
    pub continue_target: Option<usize>,
    pub break_jumps: Vec<usize>,
    pub base_locals_count: usize,
}

pub struct Generator {
    pub bytecode: Vec<OpCode>,
    pub scopes: Vec<Scope>, // A stack of lexical blocks

    pub function_registry: HashMap<String, usize>,
    pub locals_count: usize,
    pub loop_stack: Vec<LoopContext>,
    pub last_emitted_line: usize,
}

impl Generator {
    pub fn new() -> Self {
        Self {
            bytecode: Vec::new(),
            // Start with a global/root lexical scope
            scopes: vec![Scope {
                variables: HashMap::new(),
                depth_start_index: 0,
            }],

            function_registry: HashMap::new(),
            locals_count: 0,
            loop_stack: Vec::new(),
            last_emitted_line: 0,
        }
    }

    fn error(&self, message: &str, span: crate::lexer::Span) -> crate::AppError {
        crate::AppError::CompileError {
            line: span.line,
            column: span.column,
            message: message.to_string(),
        }
    }

    fn root_variable_name(expr: &ExpressionNode) -> Option<&str> {
        match &expr.kind {
            Expression::Variable { name, .. } => Some(name.as_str()),
            Expression::Index { object, .. } => Self::root_variable_name(object),
            _ => None,
        }
    }

    fn clear_temporary_statement_loans(&mut self) {
        let current_depth = self.scopes.len();
        for scope in &mut self.scopes {
            for metadata in scope.variables.values_mut() {
                metadata
                    .active_loans
                    .retain(|loan| loan.borrow_scope_depth < current_depth);
            }
        }
    }

    // Look up variable tracking metadata immutably (walking scopes backwards)
    fn resolve_variable_metadata(&self, name: &str) -> Option<&VariableMetadata> {
        for scope in self.scopes.iter().rev() {
            if let Some(metadata) = scope.variables.get(name) {
                return Some(metadata);
            }
        }
        None
    }

    /// Look up variable tracking metadata mutably so we can append/remove loans
    fn resolve_variable_metadata_mut(&mut self, name: &str) -> Option<&mut VariableMetadata> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(metadata) = scope.variables.get_mut(name) {
                return Some(metadata);
            }
        }
        None
    }

    /// Helper to inject default metadata when a new variable is initialized
    fn insert_variable_metadata(&mut self, name: String, slot: usize, is_mutable: bool) {
        let metadata = VariableMetadata {
            slot,
            is_mutable,
            //borrowing_from: None,
            active_loans: Vec::new(),
        };
        self.current_scope_mut().variables.insert(name, metadata);
    }

    fn borrow_variable(
        &mut self,
        source_name: &str,
        borrow_kind: BorrowKind,
        span: crate::lexer::Span,
    ) -> Result<(), crate::AppError> {
        let current_depth = self.scopes.len();

        if let Some(metadata) = self.resolve_variable_metadata_mut(source_name) {
            if borrow_kind == BorrowKind::Mut && !metadata.is_mutable {
                return Err(self.error(
                    &format!("Cannot implicitly borrow '{}' mutably because it was not declared with explicit 'mut' keyword.", source_name),
                    span,
                ));
            }

            for active_loan in &metadata.active_loans {
                match (active_loan.kind, borrow_kind) {
                    (BorrowKind::Mut, _) => {
                        return Err(self.error(
                            &format!("Cannot borrow '{}' because it is already mutably borrowed exclusively.", source_name),
                            span,
                        ));
                    }
                    (BorrowKind::Shared, BorrowKind::Mut) => {
                        return Err(self.error(
                            &format!("Cannot mutably borrow '{}' because it is currently read-only borrowed by active loops.", source_name),
                            span,
                        ));
                    }
                    (BorrowKind::Shared, BorrowKind::Shared) => {}
                }
            }

            metadata.active_loans.push(Loan {
                provider_name: source_name.to_string(),
                kind: borrow_kind,
                borrow_scope_depth: current_depth,
            });
        }
        Ok(())
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope {
            variables: HashMap::new(),
            depth_start_index: self.locals_count, // Safely bookmark where we started
        });
    }

    fn pop_scope(&mut self) {
        let current_depth = self.scopes.len();

        if let Some(popped_scope) = self.scopes.pop() {
            self.locals_count = popped_scope.depth_start_index;

            // Walk backwards through remaining parent scopes
            // and remove loans that were tied to the popped depth level
            for scope in &mut self.scopes {
                for metadata in scope.variables.values_mut() {
                    metadata
                        .active_loans
                        .retain(|loan| loan.borrow_scope_depth < current_depth);
                }
            }
        }
    }

    /// Main entry point helper to translate an AST into VM input parameters
    pub fn compile(
        program: &Program,
    ) -> Result<(Vec<OpCode>, HashMap<String, usize>), crate::AppError> {
        let mut generator = Generator::new();

        let native_functions = crate::nativefns::get_native_registry();

        for (index, item) in native_functions.keys().enumerate() {
            // Insert native hooks as read-only global entries
            generator.scopes[0].variables.insert(
                item.to_string(),
                VariableMetadata {
                    slot: 9999 - index,
                    is_mutable: false,
                    //borrowing_from: None,
                    active_loans: Vec::new(),
                },
            );
        }

        generator.compile_program(program)?;
        Ok(generator.finish())
    }
    // Consumes the generator and returns the generated bytecode
    // alongside the tracked user function positions.
    pub fn finish(self) -> (Vec<OpCode>, HashMap<String, usize>) {
        (self.bytecode, self.function_registry)
    }

    // Helper to output bytecode and track its current position
    fn emit(&mut self, op: OpCode) -> usize {
        self.bytecode.push(op);
        self.bytecode.len() - 1
    }

    // Resolve a string variable name to its numerical stack slot (walking scopes backwards)
    fn resolve_local(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(metadata) = scope.variables.get(name) {
                return Some(metadata.slot);
            }
        }
        None
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes
            .last_mut()
            .expect("Generator stack error: No active scope found.")
    }

    pub fn compile_program(&mut self, program: &Program) -> Result<(), crate::AppError> {
        for stmt in &program.body {
            self.compile_statement(stmt)?;
        }
        Ok(())
    }

    pub fn compile_statement(&mut self, stmt: &StatementNode) -> Result<(), crate::AppError> {
        if stmt.span.line != self.last_emitted_line {
            self.last_emitted_line = stmt.span.line;
            self.emit(OpCode::LineUpdate(stmt.span));
        }

        #[allow(unreachable_patterns)]
        match &stmt.kind {
            Statement::VarDeclaration {
                name,
                initializer,
                is_mutable,
                var_type: _,
            } => {
                self.compile_expression(initializer)?;

                let slot = self.locals_count;
                self.insert_variable_metadata(name.clone(), slot, *is_mutable);

                self.locals_count += 1;
                self.emit(OpCode::SetLocal(slot));
                self.clear_temporary_statement_loans();
            }

            Statement::Assignment { name, value } => {
                if let Some(metadata) = self.resolve_variable_metadata(name) {
                    if !metadata.is_mutable {
                        return Err(self.error(&format!("Cannot assign to variable '{}' because it was not explicitly declared as mutable with 'mut'.", name), stmt.span));
                    }

                    if !metadata.active_loans.is_empty() {
                        return Err(self.error(&format!("Cannot mutate variable '{}' because it is currently implicitly borrowed elsewhere.", name), stmt.span));
                    }
                } else {
                    return Err(self.error(
                        &format!("Variable '{}' assigned before declaration.", name),
                        stmt.span,
                    ));
                }
                self.compile_expression(value)?;
                let slot = self.resolve_local(name).unwrap();
                self.emit(OpCode::SetLocal(slot));
                self.clear_temporary_statement_loans();
            }

            Statement::CompoundAssignment {
                name,
                operator,
                value,
            } => {
                if let Some(metadata) = self.resolve_variable_metadata(name) {
                    if !metadata.is_mutable {
                        return Err(self.error(
                &format!("Cannot assign to variable '{}' because it was not explicitly declared as mutable with 'mut'.", name),
                stmt.span,
            ));
                    }
                    if !metadata.active_loans.is_empty() {
                        return Err(self.error(
                &format!("Cannot mutate variable '{}' because it is currently implicitly borrowed elsewhere.", name),
                stmt.span,
            ));
                    }
                } else {
                    return Err(self.error(
                        &format!("Variable '{}' assigned before declaration.", name),
                        stmt.span,
                    ));
                }

                self.compile_expression(value)?;
                let slot = self.resolve_local(name).unwrap();
                self.emit(OpCode::CompoundAssignLocal(slot, *operator));
                self.clear_temporary_statement_loans();
            }

            Statement::IndexAssignment {
                target,
                index,
                value,
            } => {
                if let Expression::Variable { name: obj_name, .. } = &target.kind {
                    let slot = self.resolve_local(obj_name).ok_or_else(|| {
                        self.error(
                            &format!("Variable '{}' used before declaration.", obj_name),
                            stmt.span,
                        )
                    })?;
                    self.compile_expression(index)?;
                    self.compile_expression(value)?;
                    self.clear_temporary_statement_loans();
                    self.borrow_variable(obj_name, BorrowKind::Mut, stmt.span)?;
                    self.emit(OpCode::SetLocalIndex(slot));
                    self.clear_temporary_statement_loans();
                } else {
                    self.compile_expression(target)?;
                    self.compile_expression(index)?;
                    self.compile_expression(value)?;
                    self.clear_temporary_statement_loans();

                    if let Some(root_name) = Self::root_variable_name(target) {
                        self.borrow_variable(root_name, BorrowKind::Mut, stmt.span)?;
                    }

                    self.emit(OpCode::SetIndex);
                    self.clear_temporary_statement_loans();
                }
            }

            Statement::CompoundIndexAssignment {
                target,
                index,
                value,
                operator,
            } => {
                if let Expression::Variable { name: obj_name, .. } = &target.kind {
                    let slot = self.resolve_local(obj_name).ok_or_else(|| {
                        self.error(
                            &format!("Variable '{}' used before declaration.", obj_name),
                            stmt.span,
                        )
                    })?;
                    self.compile_expression(index)?;
                    self.compile_expression(value)?;
                    self.clear_temporary_statement_loans();
                    self.borrow_variable(obj_name, BorrowKind::Mut, stmt.span)?;
                    self.emit(OpCode::CompoundAssignLocalIndex(slot, *operator));
                    self.clear_temporary_statement_loans();
                } else {
                    self.compile_expression(target)?;
                    self.compile_expression(index)?;
                    self.compile_expression(value)?;
                    self.clear_temporary_statement_loans();

                    if let Some(root_name) = Self::root_variable_name(target) {
                        self.borrow_variable(root_name, BorrowKind::Mut, stmt.span)?;
                    }

                    self.emit(OpCode::CompoundAssignIndex(*operator));
                    self.clear_temporary_statement_loans();
                }
            }

            Statement::Expression(expr) => {
                self.compile_expression(expr)?;

                // If it's a Call context that targets a method (like .append) which pushes an explicit
                // Return placeholder or handles its own stack footprint balance, make sure we don't
                // aggressively pop from an empty evaluation stack register.
                match &expr.kind {
                    Expression::Call { callee, .. } => {
                        if let Expression::Get { .. } = &callee.kind {
                            self.emit(OpCode::Pop);
                        } else {
                            self.emit(OpCode::Pop);
                        }
                    }
                    _ => {
                        self.emit(OpCode::Pop);
                    }
                }

                self.clear_temporary_statement_loans();
            }

            Statement::Block(statements) => {
                // Use the parameterless helper to keep counters aligned
                self.push_scope();
                for sub_stmt in statements {
                    self.compile_statement(sub_stmt)?;
                }
                // Use the parameterless helper to reclaim local variable stack slots
                self.pop_scope();
            }

            Statement::Match {
                target,
                cases,
                default_branch,
            } => {
                // Evaluate the target expression FIRST (pushes value to evaluation stack)
                self.compile_expression(target)?;

                let hidden_target_name = format!("$match_target_{}", self.bytecode.len());
                let target_slot = self.locals_count;
                self.locals_count += 1;

                // Store the value into the register slot immediately
                self.insert_variable_metadata(hidden_target_name.clone(), target_slot, false);
                self.emit(OpCode::SetLocal(target_slot));

                // Safely open the inner case block scope context using the helper
                self.push_scope();

                let mut end_jumps = Vec::new();
                let mut next_case_jump_ip: Option<usize> = None;

                // Process structural match cases sequentially
                for case in cases {
                    if let Some(prev_false_jump) = next_case_jump_ip {
                        let current_address = self.bytecode.len();
                        if let Some(OpCode::JumpIfFalse(target)) =
                            self.bytecode.get_mut(prev_false_jump)
                        {
                            *target = current_address;
                        }
                    }

                    self.emit(OpCode::GetLocal(target_slot));
                    self.compile_expression(&case.pattern)?;
                    self.emit(OpCode::Equal);

                    next_case_jump_ip = Some(self.emit(OpCode::JumpIfFalse(0)));

                    // Inline block elements directly to prevent double-scoping indexing misalignments
                    match &case.body.kind {
                        Statement::Block(inner_statements) => {
                            for sub_stmt in inner_statements {
                                self.compile_statement(sub_stmt)?;
                            }
                        }
                        _ => {
                            self.compile_statement(&case.body)?;
                        }
                    }

                    end_jumps.push(self.emit(OpCode::Jump(0)));
                }

                if let Some(prev_false_jump) = next_case_jump_ip {
                    let fallthrough_address = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(target)) =
                        self.bytecode.get_mut(prev_false_jump)
                    {
                        *target = fallthrough_address;
                    }
                }
                // Compile the default branch down below the escape terminal vector bounds
                if let Some(default_stmt) = default_branch {
                    self.compile_statement(default_stmt)?;
                } else {
                    self.emit(OpCode::PushNil);
                    self.emit(OpCode::Pop);
                }

                // Calculate the match end right here, BEFORE the default branch compiles
                // to ensure all successful pattern arms leap completely past the default block
                let match_end_address = self.bytecode.len();

                // Backpatch successful case paths to jump to this safe end-marker address address
                for exit_jump in end_jumps {
                    if let Some(OpCode::Jump(target)) = self.bytecode.get_mut(exit_jump) {
                        *target = match_end_address;
                    }
                }

                // Pop the match block scope context safely using the parameterless helper
                self.pop_scope();
            }

            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                // Evaluate condition
                self.compile_expression(condition)?;
                self.clear_temporary_statement_loans();

                // Emit placeholder jump
                let jump_false_ip = self.emit(OpCode::JumpIfFalse(0));

                // Process the true code pathway
                self.compile_statement(then_branch)?;

                if let Some(else_stmt) = else_branch {
                    // Skip else block if we executed then block
                    let jump_end_ip = self.emit(OpCode::Jump(0));

                    // Backpatch: false pathway starts exactly here
                    let else_start_ip = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(jump_false_ip)
                    {
                        *target = else_start_ip;
                    }

                    self.compile_statement(else_stmt)?;

                    let post_else_ip = self.bytecode.len();
                    if let Some(OpCode::Jump(target)) = self.bytecode.get_mut(jump_end_ip) {
                        *target = post_else_ip;
                    }
                } else {
                    // No else block present, backpatch directly to the exit point
                    let post_if_ip = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(jump_false_ip)
                    {
                        *target = post_if_ip;
                    }
                }
            }

            Statement::FunctionDeclaration { name, params, body } => {
                // Skip compiling function body during normal sequential execution
                let jump_over_func_ip = self.emit(OpCode::Jump(0));

                // Track where this function starts in the bytecode array
                let func_entry_point = self.bytecode.len();
                self.function_registry
                    .insert(name.clone(), func_entry_point);

                // Setup a clean local variable scope space for this function
                self.push_scope();
                self.last_emitted_line = 0;

                // Map parameters directly to slots 0, 1, 2 sequentially
                for (i, param) in params.iter().enumerate() {
                    if let Expression::Variable { name, is_mutable } = &param.kind {
                        self.insert_variable_metadata(name.clone(), i, *is_mutable);
                        self.locals_count += 1;
                    }
                }

                // Compile inner body statements while tracking the last item
                let len = body.len();
                for (idx, body_stmt) in body.iter().enumerate() {
                    let is_last_statement = idx == len - 1;

                    if is_last_statement {
                        // If the final statement is an implicit expression return
                        if let Statement::Expression(expr) = &body_stmt.kind {
                            self.compile_expression(expr)?;
                            continue; // Skip normal statement processing to bypass Pop
                        }
                    }
                    // Otherwise compile the statement normally (which will Pop if it's a mid-block expression)
                    self.compile_statement(body_stmt)?;
                }

                // Push a fallback Nil onto the stack right before the trailing Return opcode
                // To safely handle it without adding extra stack overhead:
                // We only emit PushNil if the last statement wasn't an implicit expression return.
                let ends_with_expression = !body.is_empty()
                    && matches!(body.last().unwrap().kind, Statement::Expression(_));
                if !ends_with_expression {
                    self.emit(OpCode::PushNil);
                }

                self.emit(OpCode::Return);

                // Clean up and pop the function local scope frame
                self.pop_scope();

                // Reset the local tracking metadata counts and clear the root scope variables!
                // Since functions are isolated packages, we flush previous scratchpads
                // so subsequent function passes don't inherit old loan collisions.
                if self.scopes.len() == 1 {
                    self.scopes[0].variables.clear();

                    // Re-register the native system hooks into the wiped root scope
                    let native_functions = crate::nativefns::get_native_registry();
                    for (index, item) in native_functions.keys().enumerate() {
                        self.scopes[0].variables.insert(
                            item.to_string(),
                            VariableMetadata {
                                slot: 9999 - index,
                                is_mutable: false,
                                //borrowing_from: None,
                                active_loans: Vec::new(),
                            },
                        );
                    }
                }

                // Normal outer execution lands safely past function body
                let post_func_ip = self.bytecode.len();
                if let Some(OpCode::Jump(target)) = self.bytecode.get_mut(jump_over_func_ip) {
                    *target = post_func_ip;
                }
            }

            Statement::Return(maybe_expr) => {
                if let Some(expr) = maybe_expr {
                    // If an expression exists compile it onto the evaluation stack
                    self.compile_expression(expr)?;
                } else {
                    // If it's a blank return push Nil onto the stack as a placeholder
                    self.emit(OpCode::PushNil);
                }

                self.emit(OpCode::Return);
            }
            Statement::Break(maybe_expr) => {
                if self.loop_stack.is_empty() {
                    return Err(self.error("Cannot 'break' outside of a loop.", stmt.span));
                }
                if let Some(expr) = maybe_expr {
                    self.compile_expression(expr)?;
                } else {
                    self.emit(OpCode::PushNil);
                }
                let jump_ip = self.emit(OpCode::Jump(0));
                self.loop_stack
                    .last_mut()
                    .unwrap()
                    .break_jumps
                    .push(jump_ip);
            }

            Statement::Continue => {
                let continue_target = match self.loop_stack.last() {
                    Some(ctx) => ctx.continue_target,
                    None => {
                        return Err(self.error("Cannot 'continue' outside of a loop.", stmt.span));
                    }
                };

                if let Some(target) = continue_target {
                    self.emit(OpCode::Jump(target));
                } else {
                    let jump_ip = self.emit(OpCode::Jump(0));
                    self.loop_stack
                        .last_mut()
                        .unwrap()
                        .continue_jumps
                        .push(jump_ip);
                }
            }

            _ => todo!("Got statement that is yet to be implemented."),
        }
        Ok(())
    }

    pub fn compile_expression(&mut self, expr: &ExpressionNode) -> Result<(), crate::AppError> {
        #[allow(unreachable_patterns)]
        match &expr.kind {
            Expression::Literal(lit) => match lit {
                LiteralValue::Int(n) => {
                    self.emit(OpCode::PushInt(*n));
                }
                LiteralValue::Float(n) => {
                    self.emit(OpCode::PushFloat(*n));
                }
                LiteralValue::String(s) => {
                    self.emit(OpCode::PushString(Rc::new(s.to_string())));
                }
                LiteralValue::Boolean(b) => {
                    self.emit(OpCode::PushBoolean(*b));
                }
                LiteralValue::Nil => {
                    self.emit(OpCode::PushNil);
                }
            },

            Expression::Variable { name, .. } => {
                self.borrow_variable(name, BorrowKind::Shared, expr.span)?;

                if let Some(slot) = self.resolve_local(name) {
                    self.emit(OpCode::GetLocal(slot));
                } else {
                    return Err(self.error(
                        &format!("Variable '{}' used but not declared.", name),
                        expr.span,
                    ));
                }
            }

            Expression::Binary {
                operator,
                left,
                right,
            } => {
                if matches!(operator, BinaryOperator::And) {
                    self.compile_expression(left)?;
                    let jump_if_false = self.emit(OpCode::JumpIfFalse(0));
                    self.compile_expression(right)?;
                    let jump_end = self.emit(OpCode::Jump(0));

                    let false_target = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(jump_if_false)
                    {
                        *target = false_target;
                    }
                    self.emit(OpCode::PushBoolean(false));

                    let end_target = self.bytecode.len();
                    if let Some(OpCode::Jump(target)) = self.bytecode.get_mut(jump_end) {
                        *target = end_target;
                    }
                    return Ok(());
                }

                self.compile_expression(left)?;
                self.compile_expression(right)?;
                match operator {
                    BinaryOperator::Add => {
                        self.emit(OpCode::Add);
                    }
                    BinaryOperator::Sub => {
                        self.emit(OpCode::Sub);
                    }
                    BinaryOperator::Mul => {
                        self.emit(OpCode::Mul);
                    }
                    BinaryOperator::Div => {
                        self.emit(OpCode::Div);
                    }
                    BinaryOperator::Mod => {
                        self.emit(OpCode::Mod);
                    }
                    BinaryOperator::Equal => {
                        self.emit(OpCode::Equal);
                    }
                    BinaryOperator::NotEqual => {
                        self.emit(OpCode::NotEqual);
                    }
                    BinaryOperator::LessThan => {
                        self.emit(OpCode::LessThan);
                    }
                    BinaryOperator::GreaterThan => {
                        self.emit(OpCode::GreaterThan);
                    }
                    BinaryOperator::LessEqual => {
                        self.emit(OpCode::LessEqual);
                    }
                    BinaryOperator::GreaterEqual => {
                        self.emit(OpCode::GreaterEqual);
                    }
                    BinaryOperator::And => {
                        self.emit(OpCode::And);
                    }
                    BinaryOperator::Or => {
                        self.emit(OpCode::Or);
                    }
                    _ => todo!(),
                };
            }

            Expression::Unary { operator, right } => {
                // Compile the inner expression onto the stack first
                self.compile_expression(right)?;

                // Emit the correct modification opcode based on the operator type
                match operator {
                    crate::parser::UnaryOperator::Minus => {
                        self.emit(OpCode::Negate);
                    }
                    crate::parser::UnaryOperator::Not => {
                        self.emit(OpCode::Not);
                    }
                }
            }

            Expression::Array(elements) => {
                for item in elements {
                    self.compile_expression(item)?;
                }
                self.emit(OpCode::BuildArray(elements.len()));
            }

            Expression::Call { callee, arguments } => {
                // Method Calls (x.append(y))
                if let Expression::Get { object, name } = &callee.kind {
                    let is_mutating_method = crate::nativemthds::is_mutating_method(name);

                    if let Expression::Variable { name: obj_name, .. } = &object.kind {
                        if is_mutating_method {
                            // Register implicit exclusive mutable lease
                            self.borrow_variable(obj_name, BorrowKind::Mut, expr.span)?;
                        } else {
                            // Register implicit read-only shared lease
                            self.borrow_variable(obj_name, BorrowKind::Shared, expr.span)?;
                        }

                        // Manually emit GetLocal for the variable object instead of
                        // calling compile_expression. This cuts off double-borrowing panics
                        if let Some(slot) = self.resolve_local(obj_name) {
                            self.emit(OpCode::GetLocal(slot));
                        } else {
                            return Err(self.error(
                                &format!("Variable '{}' used before declaration.", obj_name),
                                expr.span,
                            ));
                        }
                    } else {
                        // Fallback manner for raw elements ( [1, 2].get(0) )
                        self.compile_expression(object)?;
                    }

                    // Compile the parameters normally
                    for arg in arguments {
                        self.compile_expression(arg)?;
                    }

                    self.emit(OpCode::CallMethod {
                        name: Rc::new(name.clone()),
                        arg_count: arguments.len(),
                    });

                    // Return early to balance statement execution frames
                    return Ok(());
                }

                // --- Standalone Global Function Calls ( print(x), test_to_number() ) ---
                if let Expression::Variable { name, .. } = &callee.kind {
                    // Compile function parameters onto the stack
                    for arg in arguments {
                        self.compile_expression(arg)?;
                    }

                    self.emit(OpCode::CallFunction {
                        name: Rc::new(name.clone()),
                        arg_count: arguments.len(),
                    });
                } else {
                    return Err(self.error(
                        "Dynamic/Anonymous function calls are not supported yet.",
                        expr.span,
                    ));
                }
            }

            Expression::While { condition, body } => {
                let loop_start_ip = self.bytecode.len();

                self.loop_stack.push(LoopContext {
                    continue_jumps: Vec::new(),
                    continue_target: Some(loop_start_ip),
                    break_jumps: Vec::new(),
                    base_locals_count: self.locals_count,
                });

                self.compile_expression(condition)?;
                self.clear_temporary_statement_loans();

                let jump_false_ip = self.emit(OpCode::JumpIfFalse(0));
                self.compile_statement(body)?;
                self.emit(OpCode::Jump(loop_start_ip));

                // Natural loop exit: condition became false
                let natural_end_ip = self.bytecode.len();
                if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(jump_false_ip) {
                    *target = natural_end_ip;
                }
                self.emit(OpCode::PushNil);

                // Break exit: value is already on the stack, skip PushNil
                let after_loop_ip = self.bytecode.len();
                let loop_ctx = self.loop_stack.pop().unwrap();
                self.locals_count = loop_ctx.base_locals_count;

                for break_ip in loop_ctx.break_jumps {
                    if let Some(OpCode::Jump(target)) = self.bytecode.get_mut(break_ip) {
                        *target = after_loop_ip;
                    }
                }
            }

            Expression::For {
                item_name,
                iterable,
                body,
            } => {
                let hidden_idx_name = format!("$idx_{}", self.bytecode.len());
                let hidden_list_name = format!("$list_{}", self.bytecode.len());

                let mut source_iterable_var = None;
                if let Expression::Variable { name: var_name, .. } = &iterable.kind {
                    source_iterable_var = Some(var_name.clone());
                    self.borrow_variable(var_name, BorrowKind::Shared, expr.span)?;
                }

                self.push_scope();

                // Initializers
                self.compile_expression(iterable)?;
                self.clear_temporary_statement_loans();
                let list_slot = self.locals_count;
                self.locals_count += 1;
                self.insert_variable_metadata(hidden_list_name, list_slot, false);
                self.emit(OpCode::SetLocal(list_slot));

                self.emit(OpCode::PushInt(0));
                let idx_slot = self.locals_count;
                self.locals_count += 1;
                self.insert_variable_metadata(hidden_idx_name, idx_slot, true);
                self.emit(OpCode::SetLocal(idx_slot));

                // Loop Condition Guard
                let loop_start_ip = self.bytecode.len();
                self.emit(OpCode::GetLocal(idx_slot));
                self.emit(OpCode::GetLocal(list_slot));
                self.emit(OpCode::CallFunction {
                    name: Rc::new("len".to_string()),
                    arg_count: 1,
                });
                self.emit(OpCode::LessThan);

                let jump_false_ip = self.emit(OpCode::JumpIfFalse(0));

                // Inner Loop Workspace
                let item_slot = self.locals_count;
                self.locals_count += 1;
                self.insert_variable_metadata(item_name.clone(), item_slot, false);

                self.emit(OpCode::GetLocal(list_slot));
                self.emit(OpCode::GetLocal(idx_slot));
                self.emit(OpCode::GetIndex);
                self.emit(OpCode::SetLocal(item_slot));

                self.loop_stack.push(LoopContext {
                    continue_jumps: Vec::new(),
                    continue_target: None, // Backpatched right before idx + 1
                    break_jumps: Vec::new(),
                    base_locals_count: self.locals_count,
                });

                self.compile_statement(body)?;

                // Backpatch `continue` jumps to land right here before incrementing idx
                let increment_ip = self.bytecode.len();
                let loop_ctx = self.loop_stack.pop().unwrap();
                self.locals_count = loop_ctx.base_locals_count;

                for cont_ip in loop_ctx.continue_jumps {
                    if let Some(OpCode::Jump(target)) = self.bytecode.get_mut(cont_ip) {
                        *target = increment_ip;
                    }
                }

                // Loop Counter Increment
                self.emit(OpCode::GetLocal(idx_slot));
                self.emit(OpCode::PushInt(1));
                self.emit(OpCode::Add);
                self.emit(OpCode::SetLocal(idx_slot));
                self.emit(OpCode::Jump(loop_start_ip));

                // Natural loop exit
                let natural_end_ip = self.bytecode.len();
                if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(jump_false_ip) {
                    *target = natural_end_ip;
                }
                self.emit(OpCode::PushNil);

                // Break exit
                let after_loop_ip = self.bytecode.len();
                for break_ip in loop_ctx.break_jumps {
                    if let Some(OpCode::Jump(target)) = self.bytecode.get_mut(break_ip) {
                        *target = after_loop_ip;
                    }
                }

                self.pop_scope();

                if let Some(var_name) = source_iterable_var {
                    if let Some(meta) = self.resolve_variable_metadata_mut(&var_name) {
                        meta.active_loans.retain(|l| l.provider_name != var_name);
                    }
                }
            }

            Expression::Index { object, index } => {
                if let Expression::Variable { name, .. } = &object.kind {
                    self.borrow_variable(name, BorrowKind::Shared, object.span)?;
                    if let Some(slot) = self.resolve_local(name) {
                        self.compile_expression(index)?;
                        self.emit(OpCode::GetLocalIndex(slot));
                        return Ok(());
                    }
                }
                self.compile_expression(object)?;
                self.compile_expression(index)?;
                self.emit(OpCode::GetIndex);
            }

            Expression::Get { .. } => {
                return Err(self.error(
                    "Properties must be explicitly evaluated through a method call context.",
                    expr.span,
                ));
            }

            _ => todo!("Got expression that is yet to be implemented."),
        }
        Ok(())
    }
}
