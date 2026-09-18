pub(crate) mod control;
mod decl;

use crate::ast::*;
use crate::lexer::TokenType;
use crate::parser::Parser;

impl Parser {
    pub(super) fn parse_statement(&mut self) -> Result<Vec<Stmt>, String> {
        self.skip_newlines();

        if self.check_multi_assignment() {
            return self.parse_multi_assignment();
        }

        match &self.current_token().token_type {
            TokenType::Pub => self.parse_pub(),
            TokenType::Import => self.parse_import().map(|s| vec![s]),
            TokenType::From => self.parse_from_import().map(|s| vec![s]),
            TokenType::Extern => self.parse_extern().map(|s| vec![s]),
            TokenType::Struct => self.parse_struct().map(|s| vec![s]),
            TokenType::Enum => self.parse_enum().map(|s| vec![s]),
            TokenType::Const => self.parse_const(),
            TokenType::Say => self.parse_say().map(|s| vec![s]),
            TokenType::Let => self.parse_let(),
            TokenType::If => self.parse_if().map(|s| vec![s]),
            TokenType::While => self.parse_while().map(|s| vec![s]),
            TokenType::Repeat => self.parse_repeat().map(|s| vec![s]),
            TokenType::For => self.parse_for().map(|s| vec![s]),
            TokenType::Function => self.parse_function().map(|s| vec![s]),
            TokenType::Return => self.parse_return().map(|s| vec![s]),
            TokenType::Break => {
                self.advance();
                Ok(vec![Stmt::Break])
            }
            TokenType::Continue => {
                self.advance();
                Ok(vec![Stmt::Continue])
            }
            TokenType::When => self.parse_when(),
            TokenType::Try => self.parse_try_catch().map(|s| vec![s]),
            TokenType::Throw => self.parse_throw().map(|s| vec![s]),
            TokenType::Defer => self.parse_defer().map(|s| vec![s]),
            TokenType::Assert => self.parse_assert(),
            TokenType::Test
                if self.fn_depth == 0
                    && !matches!(self.peek_token().map(|t| &t.token_type), Some(TokenType::Dot)) =>
            {
                self.parse_test_or_bench(false)
            }
            TokenType::Bench
                if self.fn_depth == 0
                    && !matches!(self.peek_token().map(|t| &t.token_type), Some(TokenType::Dot)) =>
            {
                self.parse_test_or_bench(true)
            }
            TokenType::At => self.parse_attribute(),
            TokenType::Interface => self.parse_interface(),
            TokenType::Spawn => self.parse_spawn(),
            TokenType::Select => self.parse_select(),
            TokenType::Identifier(_)
            | TokenType::SelfKw
            | TokenType::Test
            | TokenType::Bench => {
                if matches!(self.current_token().token_type, TokenType::Identifier(ref s) if s == "guard") {
                    return self.parse_guard();
                }

                // Check for multi-variable assignment: a, b = 1, 2
                if self.check_multi_assignment() {
                    return self.parse_multi_assignment();
                }

                // Could be assignment or function call
                let start_pos = self.position;
                let ident = match &self.current_token().token_type {
                    TokenType::Identifier(s) => s.clone(),
                    TokenType::SelfKw => "self".to_string(),
                    TokenType::Test => "test".to_string(),
                    TokenType::Bench => "bench".to_string(),
                    _ => unreachable!(),
                };
                self.advance();

                if matches!(
                    self.current_token().token_type,
                    TokenType::LeftBracket | TokenType::Dot
                ) {
                    let mut target = Expr::Identifier(ident);
                    while matches!(
                        self.current_token().token_type,
                        TokenType::LeftBracket | TokenType::Dot
                    ) {
                        if matches!(self.current_token().token_type, TokenType::LeftBracket) {
                            self.advance();
                            let index = self.parse_expression()?;
                            self.expect(TokenType::RightBracket)?;
                            target = Expr::Index {
                                array: Box::new(target),
                                index: Box::new(index),
                            };
                        } else if matches!(self.current_token().token_type, TokenType::Dot) {
                            self.advance();
                            let field = match &self.current_token().token_type {
                                TokenType::Identifier(s) => s.clone(),
                                tok => {
                                    let s = tok.to_string();
                                    let clean = s.trim_matches('\'').to_string();
                                    if !clean.is_empty()
                                        && clean.chars().all(|c| c.is_alphanumeric() || c == '_')
                                    {
                                        clean
                                    } else {
                                        return Err(format!(
                                            "Expected field name after '.' at line {}, column {}",
                                            self.current_token().line,
                                            self.current_token().column
                                        ));
                                    }
                                }
                            };
                            self.advance();
                            if matches!(self.current_token().token_type, TokenType::LeftParen) {
                                self.advance();
                                let mut args = vec![target];
                                if !matches!(self.current_token().token_type, TokenType::RightParen)
                                {
                                    loop {
                                        args.push(self.parse_call_argument()?);
                                        if matches!(
                                            self.current_token().token_type,
                                            TokenType::Comma
                                        ) {
                                            self.advance();
                                        } else {
                                            break;
                                        }
                                    }
                                }
                                self.expect(TokenType::RightParen)?;
                                target = Expr::Call { name: field, args };
                            } else {
                                target = Expr::FieldAccess {
                                    object: Box::new(target),
                                    field,
                                };
                            }
                        }
                    }

                    match self.current_token().token_type {
                        TokenType::Assign => {
                            self.advance();
                            let value = self.parse_expression()?;
                            match target {
                                Expr::Index { array, index } => {
                                    return Ok(vec![Stmt::IndexAssign {
                                        array: *array,
                                        index: *index,
                                        value,
                                    }]);
                                }
                                Expr::FieldAccess { object, field } => {
                                    return Ok(vec![Stmt::FieldAssign {
                                        object: *object,
                                        field,
                                        value,
                                    }]);
                                }
                                _ => unreachable!(),
                            }
                        }
                        TokenType::PlusAssign
                        | TokenType::MinusAssign
                        | TokenType::MultiplyAssign
                        | TokenType::DivideAssign
                        | TokenType::ModuloAssign
                        | TokenType::BitAndAssign
                        | TokenType::BitOrAssign
                        | TokenType::BitXorAssign
                        | TokenType::ShlAssign
                        | TokenType::ShrAssign => {
                            let bin_op = match self.current_token().token_type {
                                TokenType::PlusAssign => BinaryOp::Add,
                                TokenType::MinusAssign => BinaryOp::Subtract,
                                TokenType::MultiplyAssign => BinaryOp::Multiply,
                                TokenType::DivideAssign => BinaryOp::Divide,
                                TokenType::ModuloAssign => BinaryOp::Modulo,
                                TokenType::BitAndAssign => BinaryOp::BitAnd,
                                TokenType::BitOrAssign => BinaryOp::BitOr,
                                TokenType::BitXorAssign => BinaryOp::BitXor,
                                TokenType::ShlAssign => BinaryOp::Shl,
                                TokenType::ShrAssign => BinaryOp::Shr,
                                _ => unreachable!(),
                            };
                            self.advance();
                            let value = self.parse_expression()?;
                            let bin_val = Expr::Binary {
                                left: Box::new(target.clone()),
                                op: bin_op,
                                right: Box::new(value),
                            };
                            match target {
                                Expr::Index { array, index } => {
                                    return Ok(vec![Stmt::IndexAssign {
                                        array: *array,
                                        index: *index,
                                        value: bin_val,
                                    }]);
                                }
                                Expr::FieldAccess { object, field } => {
                                    return Ok(vec![Stmt::FieldAssign {
                                        object: *object,
                                        field,
                                        value: bin_val,
                                    }]);
                                }
                                _ => unreachable!(),
                            }
                        }
                        _ => {
                            self.position = start_pos;
                            let expr = self.parse_expression()?;
                            return Ok(vec![Stmt::Expr(expr)]);
                        }
                    }
                }

                match self.current_token().token_type {
                    TokenType::Assign => {
                        self.advance();
                        let value = self.parse_expression()?;
                        Ok(vec![Stmt::Assign { name: ident, value }])
                    }
                    TokenType::PlusAssign => {
                        self.advance();
                        let value = self.parse_expression()?;
                        Ok(vec![Stmt::Assign {
                            name: ident.clone(),
                            value: Expr::Binary {
                                left: Box::new(Expr::Identifier(ident)),
                                op: BinaryOp::Add,
                                right: Box::new(value),
                            },
                        }])
                    }
                    TokenType::MinusAssign => {
                        self.advance();
                        let value = self.parse_expression()?;
                        Ok(vec![Stmt::Assign {
                            name: ident.clone(),
                            value: Expr::Binary {
                                left: Box::new(Expr::Identifier(ident)),
                                op: BinaryOp::Subtract,
                                right: Box::new(value),
                            },
                        }])
                    }
                    TokenType::MultiplyAssign => {
                        self.advance();
                        let value = self.parse_expression()?;
                        Ok(vec![Stmt::Assign {
                            name: ident.clone(),
                            value: Expr::Binary {
                                left: Box::new(Expr::Identifier(ident)),
                                op: BinaryOp::Multiply,
                                right: Box::new(value),
                            },
                        }])
                    }
                    TokenType::DivideAssign => {
                        self.advance();
                        let value = self.parse_expression()?;
                        Ok(vec![Stmt::Assign {
                            name: ident.clone(),
                            value: Expr::Binary {
                                left: Box::new(Expr::Identifier(ident)),
                                op: BinaryOp::Divide,
                                right: Box::new(value),
                            },
                        }])
                    }
                    TokenType::ModuloAssign => {
                        self.advance();
                        let value = self.parse_expression()?;
                        Ok(vec![Stmt::Assign {
                            name: ident.clone(),
                            value: Expr::Binary {
                                left: Box::new(Expr::Identifier(ident)),
                                op: BinaryOp::Modulo,
                                right: Box::new(value),
                            },
                        }])
                    }
                    TokenType::BitAndAssign => {
                        self.advance();
                        let value = self.parse_expression()?;
                        Ok(vec![Stmt::Assign {
                            name: ident.clone(),
                            value: Expr::Binary {
                                left: Box::new(Expr::Identifier(ident)),
                                op: BinaryOp::BitAnd,
                                right: Box::new(value),
                            },
                        }])
                    }
                    TokenType::BitOrAssign => {
                        self.advance();
                        let value = self.parse_expression()?;
                        Ok(vec![Stmt::Assign {
                            name: ident.clone(),
                            value: Expr::Binary {
                                left: Box::new(Expr::Identifier(ident)),
                                op: BinaryOp::BitOr,
                                right: Box::new(value),
                            },
                        }])
                    }
                    TokenType::BitXorAssign => {
                        self.advance();
                        let value = self.parse_expression()?;
                        Ok(vec![Stmt::Assign {
                            name: ident.clone(),
                            value: Expr::Binary {
                                left: Box::new(Expr::Identifier(ident)),
                                op: BinaryOp::BitXor,
                                right: Box::new(value),
                            },
                        }])
                    }
                    TokenType::ShlAssign => {
                        self.advance();
                        let value = self.parse_expression()?;
                        Ok(vec![Stmt::Assign {
                            name: ident.clone(),
                            value: Expr::Binary {
                                left: Box::new(Expr::Identifier(ident)),
                                op: BinaryOp::Shl,
                                right: Box::new(value),
                            },
                        }])
                    }
                    TokenType::ShrAssign => {
                        self.advance();
                        let value = self.parse_expression()?;
                        Ok(vec![Stmt::Assign {
                            name: ident.clone(),
                            value: Expr::Binary {
                                left: Box::new(Expr::Identifier(ident)),
                                op: BinaryOp::Shr,
                                right: Box::new(value),
                            },
                        }])
                    }
                    _ => {
                        // Put the identifier back into an expression
                        self.position = start_pos;
                        let expr = self.parse_expression()?;
                        Ok(vec![Stmt::Expr(expr)])
                    }
                }
            }
            _ => {
                let expr = self.parse_expression()?;
                Ok(vec![Stmt::Expr(expr)])
            }
        }
    }

    fn check_multi_assignment(&self) -> bool {
        let mut idx = self.position;
        let mut in_parens = false;
        if let Some(tok) = self.tokens.get(idx) {
            if matches!(tok.token_type, TokenType::LeftParen) {
                in_parens = true;
                idx += 1;
            }
        } else {
            return false;
        }

        match self.tokens.get(idx).map(|t| &t.token_type) {
            Some(TokenType::Identifier(_)) => idx += 1,
            _ => return false,
        }

        if !matches!(
            self.tokens.get(idx).map(|t| &t.token_type),
            Some(TokenType::Comma)
        ) {
            return false;
        }

        while matches!(
            self.tokens.get(idx).map(|t| &t.token_type),
            Some(TokenType::Comma)
        ) {
            idx += 1;
            match self.tokens.get(idx).map(|t| &t.token_type) {
                Some(TokenType::Identifier(_)) => idx += 1,
                _ => return false,
            }
        }

        if in_parens {
            if !matches!(
                self.tokens.get(idx).map(|t| &t.token_type),
                Some(TokenType::RightParen)
            ) {
                return false;
            }
            idx += 1;
        }

        matches!(
            self.tokens.get(idx).map(|t| &t.token_type),
            Some(TokenType::Assign)
        )
    }

    pub(super) fn parse_multi_assignment(&mut self) -> Result<Vec<Stmt>, String> {
        let first_line = self.current_token().line;
        let first_col = self.current_token().column;

        let has_parens = if matches!(self.current_token().token_type, TokenType::LeftParen) {
            self.advance();
            true
        } else {
            false
        };

        let mut names = Vec::new();
        loop {
            let name = match &self.current_token().token_type {
                TokenType::Identifier(s) => s.clone(),
                _ => unreachable!(),
            };
            self.advance();
            names.push(name);

            if matches!(self.current_token().token_type, TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        if has_parens {
            self.expect(TokenType::RightParen)?;
        }

        self.expect(TokenType::Assign)?;

        let mut values = Vec::new();
        loop {
            let value = self.parse_expression()?;
            values.push(value);

            if matches!(self.current_token().token_type, TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        if values.len() == 1 {
            let single_val = values.remove(0);
            let tmp_name = format!("__tuple_assign_{}_{}", first_line, first_col);
            let mut stmts = vec![Stmt::Let {
                name: tmp_name.clone(),
                type_ann: None,
                value: single_val,
            }];
            for (i, name) in names.into_iter().enumerate() {
                stmts.push(Stmt::Assign {
                    name,
                    value: Expr::Index {
                        array: Box::new(Expr::Identifier(tmp_name.clone())),
                        index: Box::new(Expr::Number(i as f64)),
                    },
                });
            }
            Ok(stmts)
        } else if values.len() == names.len() {
            let mut stmts = Vec::new();
            let mut tmp_names = Vec::new();
            for (i, val) in values.into_iter().enumerate() {
                let tmp_name = format!("__assign_tmp_{}_{}_{}", first_line, first_col, i);
                stmts.push(Stmt::Let {
                    name: tmp_name.clone(),
                    type_ann: None,
                    value: val,
                });
                tmp_names.push(tmp_name);
            }
            for (name, tmp) in names.into_iter().zip(tmp_names) {
                stmts.push(Stmt::Assign {
                    name,
                    value: Expr::Identifier(tmp),
                });
            }
            Ok(stmts)
        } else {
            Err(format!(
                "Mismatch in assignment: {} variables but {} values provided at line {}, column {}",
                names.len(),
                values.len(),
                first_line,
                first_col
            ))
        }
    }

    fn parse_defer(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'defer'
        let stmts = self.parse_statement()?;
        if let Some(inner) = stmts.into_iter().next() {
            Ok(Stmt::Defer(Box::new(inner)))
        } else {
            Err("Expected statement after 'defer'".into())
        }
    }

    fn parse_assert(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // consume 'assert'
        let condition = self.parse_expression()?;
        let message = if matches!(self.current_token().token_type, TokenType::Comma) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        let throw_msg = message.unwrap_or_else(|| Expr::String("Assertion failed".to_string()));
        let assert_check = Stmt::If {
            condition: Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(condition),
            },
            then_block: vec![Stmt::Throw(Some(throw_msg))],
            else_block: None,
        };
        Ok(vec![assert_check])
    }

    fn parse_test_or_bench(&mut self, is_bench: bool) -> Result<Vec<Stmt>, String> {
        self.advance(); // consume 'test' or 'bench'
        let name_expr = self.parse_expression()?;
        self.skip_newlines();
        let mut body = Vec::new();
        while !matches!(self.current_token().token_type, TokenType::End | TokenType::Eof) {
            body.extend(self.parse_statement()?);
            self.skip_newlines();
        }
        self.expect(TokenType::End)?;
        let prefix = if is_bench { "__bench_" } else { "__test_" };
        let clean_name = match &name_expr {
            Expr::String(s) => s
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect::<String>(),
            _ => format!("{}_{}", self.current_token().line, self.current_token().column),
        };
        let fn_stmt = Stmt::Function {
            name: format!("{}{}", prefix, clean_name),
            params: vec![],
            param_types: vec![],
            return_type: None,
            defaults: vec![],
            body,
        };
        Ok(vec![fn_stmt])
    }

    fn parse_attribute(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // consume '@'
        let _attr_name = match &self.current_token().token_type {
            TokenType::Identifier(s) => s.clone(),
            tok => {
                let s = tok.to_string();
                let clean = s.trim_matches('\'').to_string();
                if !clean.is_empty() && clean.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    clean
                } else {
                    return Err(format!(
                        "Expected attribute name after '@' at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ));
                }
            }
        };
        self.advance();
        if matches!(self.current_token().token_type, TokenType::LeftParen) {
            self.advance();
            let mut depth = 1;
            while depth > 0 && !matches!(self.current_token().token_type, TokenType::Eof) {
                if matches!(self.current_token().token_type, TokenType::LeftParen) {
                    depth += 1;
                } else if matches!(self.current_token().token_type, TokenType::RightParen) {
                    depth -= 1;
                    if depth == 0 {
                        self.advance();
                        break;
                    }
                }
                self.advance();
            }
        }
        self.skip_newlines();
        self.parse_statement()
    }

    fn parse_spawn(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // consume 'spawn'
        let expr = self.parse_expression()?;
        match expr {
            Expr::Call { name, args } => {
                if args.is_empty() {
                    Ok(vec![Stmt::Expr(Expr::Call {
                        name: "spawn".to_string(),
                        args: vec![Expr::Identifier(name), Expr::Null],
                    })])
                } else if args.len() == 1 {
                    Ok(vec![Stmt::Expr(Expr::Call {
                        name: "spawn".to_string(),
                        args: vec![Expr::Identifier(name), args.into_iter().next().unwrap()],
                    })])
                } else {
                    let thunk_name = format!("__alya_spawn_thunk_{}", self.lambda_counter);
                    self.lambda_counter += 1;
                    let pack_ident = "__pack".to_string();
                    let thunk_args: Vec<Expr> = (0..args.len())
                        .map(|i| Expr::Index {
                            array: Box::new(Expr::Identifier(pack_ident.clone())),
                            index: Box::new(Expr::Number(i as f64)),
                        })
                        .collect();
                    let thunk_fn = Stmt::Function {
                        name: thunk_name.clone(),
                        params: vec![pack_ident],
                        param_types: vec![None],
                        return_type: None,
                        defaults: vec![None],
                        body: vec![Stmt::Expr(Expr::Call {
                            name,
                            args: thunk_args,
                        })],
                    };
                    self.lambda_functions.push(thunk_fn);
                    Ok(vec![Stmt::Expr(Expr::Call {
                        name: "spawn".to_string(),
                        args: vec![Expr::Identifier(thunk_name), Expr::Array(args)],
                    })])
                }
            }
            other => Ok(vec![Stmt::Expr(Expr::Call {
                name: "spawn".to_string(),
                args: vec![other, Expr::Null],
            })]),
        }
    }

    fn parse_select(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // consume 'select'
        self.skip_newlines();

        struct SelectCase {
            var_name: Option<String>,
            ch_op: Expr,
            body: Vec<Stmt>,
        }

        let mut cases: Vec<SelectCase> = Vec::new();
        let mut timeout_clause: Option<(Expr, Vec<Stmt>)> = None;
        let mut else_clause: Option<Vec<Stmt>> = None;

        while !matches!(self.current_token().token_type, TokenType::End | TokenType::Eof) {
            self.skip_newlines();
            if matches!(self.current_token().token_type, TokenType::End | TokenType::Eof) {
                break;
            }

            if matches!(self.current_token().token_type, TokenType::Identifier(ref s) if s == "case") {
                self.advance(); // consume 'case'
                let var_name = if matches!(self.current_token().token_type, TokenType::Identifier(_))
                    && matches!(self.peek_token().map(|t| &t.token_type), Some(TokenType::Assign))
                {
                    let name = match &self.current_token().token_type {
                        TokenType::Identifier(s) => s.clone(),
                        _ => unreachable!(),
                    };
                    self.advance(); // identifier
                    self.advance(); // '='
                    Some(name)
                } else {
                    None
                };

                let mut ch_op = self.parse_expression()?;
                if let Expr::Call { ref mut name, .. } = ch_op {
                    if name == "recv" {
                        *name = "try_recv".to_string();
                    }
                }
                self.skip_newlines();

                let mut body = Vec::new();
                while !matches!(
                    self.current_token().token_type,
                    TokenType::End | TokenType::Else | TokenType::Eof
                ) {
                    if matches!(self.current_token().token_type, TokenType::Identifier(ref s) if s == "case" || s == "timeout") {
                        break;
                    }
                    body.extend(self.parse_statement()?);
                    self.skip_newlines();
                }

                cases.push(SelectCase { var_name, ch_op, body });
            } else if matches!(self.current_token().token_type, TokenType::Identifier(ref s) if s == "timeout") {
                self.advance(); // consume 'timeout'
                let timeout_expr = self.parse_expression()?;
                self.skip_newlines();

                let mut body = Vec::new();
                while !matches!(
                    self.current_token().token_type,
                    TokenType::End | TokenType::Else | TokenType::Eof
                ) {
                    if matches!(self.current_token().token_type, TokenType::Identifier(ref s) if s == "case" || s == "timeout") {
                        break;
                    }
                    body.extend(self.parse_statement()?);
                    self.skip_newlines();
                }

                timeout_clause = Some((timeout_expr, body));
            } else if matches!(self.current_token().token_type, TokenType::Else) {
                self.advance(); // consume 'else'
                self.skip_newlines();

                let mut body = Vec::new();
                while !matches!(self.current_token().token_type, TokenType::End | TokenType::Eof) {
                    body.extend(self.parse_statement()?);
                    self.skip_newlines();
                }

                else_clause = Some(body);
            } else {
                return Err(format!(
                    "Unexpected token in select statement: {:?}",
                    self.current_token()
                ));
            }
        }

        self.expect(TokenType::End)?;

        let sel_id = self.lambda_counter;
        self.lambda_counter += 1;
        let matched_var = format!("__sel_matched_{}", sel_id);
        let start_var = format!("__sel_start_{}", sel_id);
        let timeout_var = format!("__sel_timeout_{}", sel_id);

        let mut stmts = Vec::new();
        stmts.push(Stmt::Let {
            name: matched_var.clone(),
            type_ann: Some("int".into()),
            value: Expr::Number(0.0),
        });

        if let Some((ref t_expr, _)) = timeout_clause {
            stmts.push(Stmt::Let {
                name: start_var.clone(),
                type_ann: Some("int".into()),
                value: Expr::Call {
                    name: "clock_ms".into(),
                    args: vec![],
                },
            });
            stmts.push(Stmt::Let {
                name: timeout_var.clone(),
                type_ann: Some("int".into()),
                value: t_expr.clone(),
            });
        }

        let mut loop_body = Vec::new();
        for (i, c) in cases.iter().enumerate() {
            let val_var = format!("__sel_val_{}_{}", sel_id, i);
            let mut case_inner = Vec::new();
            case_inner.push(Stmt::Let {
                name: val_var.clone(),
                type_ann: None,
                value: c.ch_op.clone(),
            });
            let mut if_val_present = Vec::new();
            if let Some(ref vname) = c.var_name {
                if_val_present.push(Stmt::Let {
                    name: vname.clone(),
                    type_ann: None,
                    value: Expr::Identifier(val_var.clone()),
                });
            }
            if_val_present.extend(c.body.clone());
            if_val_present.push(Stmt::Assign {
                name: matched_var.clone(),
                value: Expr::Number(1.0),
            });
            case_inner.push(Stmt::If {
                condition: Expr::Binary {
                    left: Box::new(Expr::Identifier(val_var)),
                    op: BinaryOp::NotEqual,
                    right: Box::new(Expr::Null),
                },
                then_block: if_val_present,
                else_block: None,
            });

            loop_body.push(Stmt::If {
                condition: Expr::Binary {
                    left: Box::new(Expr::Identifier(matched_var.clone())),
                    op: BinaryOp::Equal,
                    right: Box::new(Expr::Number(0.0)),
                },
                then_block: case_inner,
                else_block: None,
            });
        }

        let mut after_cases = Vec::new();
        if let Some((_, ref t_body)) = timeout_clause {
            let elapsed = Expr::Binary {
                left: Box::new(Expr::Call {
                    name: "clock_ms".into(),
                    args: vec![],
                }),
                op: BinaryOp::Subtract,
                right: Box::new(Expr::Identifier(start_var)),
            };
            let timed_out_cond = Expr::Binary {
                left: Box::new(elapsed),
                op: BinaryOp::GreaterEqual,
                right: Box::new(Expr::Identifier(timeout_var)),
            };
            let mut timeout_block = t_body.clone();
            timeout_block.push(Stmt::Assign {
                name: matched_var.clone(),
                value: Expr::Number(1.0),
            });
            let else_block = vec![Stmt::Expr(Expr::Call {
                name: "sleep".into(),
                args: vec![Expr::Number(1.0)],
            })];
            after_cases.push(Stmt::If {
                condition: timed_out_cond,
                then_block: timeout_block,
                else_block: Some(else_block),
            });
        } else if let Some(ref e_body) = else_clause {
            let mut non_blocking_block = e_body.clone();
            non_blocking_block.push(Stmt::Assign {
                name: matched_var.clone(),
                value: Expr::Number(1.0),
            });
            after_cases.extend(non_blocking_block);
        } else {
            after_cases.push(Stmt::Expr(Expr::Call {
                name: "sleep".into(),
                args: vec![Expr::Number(1.0)],
            }));
        }

        loop_body.push(Stmt::If {
            condition: Expr::Binary {
                left: Box::new(Expr::Identifier(matched_var.clone())),
                op: BinaryOp::Equal,
                right: Box::new(Expr::Number(0.0)),
            },
            then_block: after_cases,
            else_block: None,
        });

        stmts.push(Stmt::While {
            condition: Expr::Binary {
                left: Box::new(Expr::Identifier(matched_var)),
                op: BinaryOp::Equal,
                right: Box::new(Expr::Number(0.0)),
            },
            body: loop_body,
        });

        Ok(stmts)
    }

    fn parse_guard(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // consume 'guard'
        if matches!(self.current_token().token_type, TokenType::Let) {
            self.advance(); // consume 'let'
            let var_name = match &self.current_token().token_type {
                TokenType::Identifier(s) => s.clone(),
                _ => return Err("Expected identifier after 'guard let'".into()),
            };
            self.advance();
            self.expect(TokenType::Assign)?;
            let expr = self.parse_expression()?;
            self.expect(TokenType::Else)?;
            self.skip_newlines();
            let mut else_stmts = Vec::new();
            while !matches!(self.current_token().token_type, TokenType::End | TokenType::Eof) {
                else_stmts.extend(self.parse_statement()?);
                self.skip_newlines();
            }
            self.expect(TokenType::End)?;

            let let_stmt = Stmt::Let {
                name: var_name.clone(),
                type_ann: None,
                value: expr,
            };
            let if_stmt = Stmt::If {
                condition: Expr::Binary {
                    left: Box::new(Expr::Identifier(var_name)),
                    op: BinaryOp::Equal,
                    right: Box::new(Expr::Null),
                },
                then_block: else_stmts,
                else_block: None,
            };
            Ok(vec![let_stmt, if_stmt])
        } else {
            let cond = self.parse_expression()?;
            self.expect(TokenType::Else)?;
            self.skip_newlines();
            let mut else_stmts = Vec::new();
            while !matches!(self.current_token().token_type, TokenType::End | TokenType::Eof) {
                else_stmts.extend(self.parse_statement()?);
                self.skip_newlines();
            }
            self.expect(TokenType::End)?;
            let if_stmt = Stmt::If {
                condition: Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(cond),
                },
                then_block: else_stmts,
                else_block: None,
            };
            Ok(vec![if_stmt])
        }
    }
}
