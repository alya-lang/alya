mod control;
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
            TokenType::Identifier(_) => {
                // Could be assignment or function call
                let start_pos = self.position;
                let ident = match &self.current_token().token_type {
                    TokenType::Identifier(s) => s.clone(),
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
                                _ => {
                                    return Err(format!(
                                        "Expected field name after '.' at line {}, column {}",
                                        self.current_token().line,
                                        self.current_token().column
                                    ))
                                }
                            };
                            self.advance();
                            if matches!(self.current_token().token_type, TokenType::LeftParen) {
                                self.advance();
                                let mut args = vec![target];
                                if !matches!(self.current_token().token_type, TokenType::RightParen)
                                {
                                    loop {
                                        args.push(self.parse_expression()?);
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
}
