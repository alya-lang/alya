use crate::ast::*;
use crate::lexer::TokenType;
use crate::parser::Parser;

enum WhenPattern {
    Exact(Expr),
    Range(Expr, Expr),
    Relational(BinaryOp, Expr),
}

fn build_pattern_condition(subject: &Expr, pattern: WhenPattern) -> Expr {
    match pattern {
        WhenPattern::Exact(expr) => Expr::Binary {
            left: Box::new(subject.clone()),
            op: BinaryOp::Equal,
            right: Box::new(expr),
        },
        WhenPattern::Range(start, end) => {
            let gte = Expr::Binary {
                left: Box::new(subject.clone()),
                op: BinaryOp::GreaterEqual,
                right: Box::new(start),
            };
            let lte = Expr::Binary {
                left: Box::new(subject.clone()),
                op: BinaryOp::LessEqual,
                right: Box::new(end),
            };
            Expr::Binary {
                left: Box::new(gte),
                op: BinaryOp::And,
                right: Box::new(lte),
            }
        }
        WhenPattern::Relational(op, expr) => Expr::Binary {
            left: Box::new(subject.clone()),
            op,
            right: Box::new(expr),
        },
    }
}

fn build_when_condition(subject: &Expr, mut patterns: Vec<WhenPattern>) -> Expr {
    let first = patterns.remove(0);
    let mut cond = build_pattern_condition(subject, first);
    for pat in patterns {
        let next_cond = build_pattern_condition(subject, pat);
        cond = Expr::Binary {
            left: Box::new(cond),
            op: BinaryOp::Or,
            right: Box::new(next_cond),
        };
    }
    cond
}

impl Parser {
    pub(super) fn parse_if(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'if'
        let condition = self.parse_expression()?;
        self.skip_newlines();

        let mut then_block = Vec::new();
        while !matches!(
            self.current_token().token_type,
            TokenType::Else | TokenType::Elif | TokenType::End | TokenType::Eof
        ) {
            then_block.extend(self.parse_statement()?);
            self.skip_newlines();
        }

        let (else_block, is_chained) = if matches!(self.current_token().token_type, TokenType::Elif)
        {
            let else_if = self.parse_if()?;
            (Some(vec![else_if]), true)
        } else if matches!(self.current_token().token_type, TokenType::Else) {
            self.advance();

            // Check for 'else if'
            if matches!(self.current_token().token_type, TokenType::If) {
                let else_if = self.parse_if()?;
                (Some(vec![else_if]), true)
            } else {
                self.skip_newlines();
                let mut else_stmts = Vec::new();
                while !matches!(
                    self.current_token().token_type,
                    TokenType::End | TokenType::Eof
                ) {
                    else_stmts.extend(self.parse_statement()?);
                    self.skip_newlines();
                }
                (Some(else_stmts), false)
            }
        } else {
            (None, false)
        };

        if !is_chained {
            if !matches!(self.current_token().token_type, TokenType::End) {
                return Err(format!(
                    "Expected 'end' at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ));
            }
            self.advance();
        }

        Ok(Stmt::If {
            condition,
            then_block,
            else_block,
        })
    }

    pub(super) fn parse_while(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'while'
        let condition = self.parse_expression()?;
        self.skip_newlines();

        let mut body = Vec::new();
        while !matches!(
            self.current_token().token_type,
            TokenType::End | TokenType::Eof
        ) {
            body.extend(self.parse_statement()?);
            self.skip_newlines();
        }

        self.expect(TokenType::End)?;

        Ok(Stmt::While { condition, body })
    }

    pub(super) fn parse_repeat(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'repeat'
        self.skip_newlines();

        let mut body = Vec::new();
        while !matches!(
            self.current_token().token_type,
            TokenType::End | TokenType::Eof
        ) {
            body.extend(self.parse_statement()?);
            self.skip_newlines();
        }

        self.expect(TokenType::End)?;

        Ok(Stmt::Repeat { body })
    }

    pub(super) fn parse_for(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'for'

        let var = match &self.current_token().token_type {
            TokenType::Identifier(s) => s.clone(),
            _ => {
                return Err(format!(
                    "Expected identifier after 'for' at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ))
            }
        };
        self.advance();

        let value_var = if matches!(self.current_token().token_type, TokenType::Comma) {
            self.advance();
            let v2 = match &self.current_token().token_type {
                TokenType::Identifier(s) => s.clone(),
                _ => {
                    return Err(format!(
                        "Expected second identifier after ',' in 'for' at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ))
                }
            };
            self.advance();
            Some(v2)
        } else {
            None
        };

        self.expect(TokenType::In)?;

        let expr = self.parse_expression()?;
        if matches!(self.current_token().token_type, TokenType::DotDot) {
            if value_var.is_some() {
                return Err(format!(
                    "Multiple loop variables are not supported for range loops at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ));
            }
            self.advance();
            let end = self.parse_expression()?;
            self.skip_newlines();

            let mut body = Vec::new();
            while !matches!(
                self.current_token().token_type,
                TokenType::End | TokenType::Eof
            ) {
                body.extend(self.parse_statement()?);
                self.skip_newlines();
            }

            self.expect(TokenType::End)?;

            Ok(Stmt::For {
                var,
                start: expr,
                end,
                body,
            })
        } else {
            self.skip_newlines();

            let mut body = Vec::new();
            while !matches!(
                self.current_token().token_type,
                TokenType::End | TokenType::Eof
            ) {
                body.extend(self.parse_statement()?);
                self.skip_newlines();
            }

            self.expect(TokenType::End)?;

            Ok(Stmt::ForEach {
                var,
                value_var,
                iterable: expr,
                body,
            })
        }
    }

    pub(super) fn parse_when(&mut self) -> Result<Vec<Stmt>, String> {
        let when_line = self.current_token().line;
        let when_col = self.current_token().column;
        self.advance(); // skip 'when'
        let raw_subject = self.parse_expression()?;
        self.skip_newlines();

        let (subject, init_stmt) = match &raw_subject {
            Expr::Identifier(_) => (raw_subject, None),
            _ => {
                let temp_name = format!("__when_subj_{}", self.position);
                let let_stmt = Stmt::Let {
                    name: temp_name.clone(),
                    value: raw_subject,
                };
                (Expr::Identifier(temp_name), Some(let_stmt))
            }
        };

        let mut arms: Vec<(Vec<WhenPattern>, Vec<Stmt>)> = Vec::new();
        let mut else_block = None;

        while !matches!(
            self.current_token().token_type,
            TokenType::End | TokenType::Eof
        ) {
            if matches!(self.current_token().token_type, TokenType::Is) {
                self.advance(); // skip 'is'
                self.skip_newlines();

                let mut patterns = Vec::new();
                loop {
                    let rel_op = match self.current_token().token_type {
                        TokenType::Greater => Some(BinaryOp::Greater),
                        TokenType::Less => Some(BinaryOp::Less),
                        TokenType::GreaterEqual => Some(BinaryOp::GreaterEqual),
                        TokenType::LessEqual => Some(BinaryOp::LessEqual),
                        TokenType::NotEqual => Some(BinaryOp::NotEqual),
                        TokenType::Equal => Some(BinaryOp::Equal),
                        _ => None,
                    };

                    if let Some(op) = rel_op {
                        self.advance();
                        let expr = self.parse_expression()?;
                        patterns.push(WhenPattern::Relational(op, expr));
                    } else {
                        let pattern_start = self.parse_expression()?;
                        if matches!(self.current_token().token_type, TokenType::DotDot) {
                            self.advance(); // skip '..'
                            let pattern_end = self.parse_expression()?;
                            patterns.push(WhenPattern::Range(pattern_start, pattern_end));
                        } else {
                            patterns.push(WhenPattern::Exact(pattern_start));
                        }
                    }

                    if matches!(self.current_token().token_type, TokenType::Comma) {
                        self.advance(); // skip ','
                        self.skip_newlines();
                    } else {
                        break;
                    }
                }

                // Optional 'then'
                if matches!(self.current_token().token_type, TokenType::Then) {
                    self.advance();
                }
                self.skip_newlines();

                let mut arm_stmts = Vec::new();
                while !matches!(
                    self.current_token().token_type,
                    TokenType::Is | TokenType::Else | TokenType::End | TokenType::Eof
                ) {
                    arm_stmts.extend(self.parse_statement()?);
                    self.skip_newlines();
                }
                arms.push((patterns, arm_stmts));
            } else if matches!(self.current_token().token_type, TokenType::Else) {
                self.advance(); // skip 'else'
                if matches!(self.current_token().token_type, TokenType::Then) {
                    self.advance();
                }
                self.skip_newlines();
                let mut else_stmts = Vec::new();
                while !matches!(
                    self.current_token().token_type,
                    TokenType::End | TokenType::Eof
                ) {
                    else_stmts.extend(self.parse_statement()?);
                    self.skip_newlines();
                }
                else_block = Some(else_stmts);
                break;
            } else {
                return Err(format!(
                    "Expected 'is' or 'else' in 'when' block at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ));
            }
        }

        self.expect(TokenType::End)?;

        if arms.is_empty() && else_block.is_none() {
            return Err(format!(
                "Empty 'when' statement at line {}, column {}",
                when_line, when_col
            ));
        }

        // Desugar when into nested If statements
        let mut current_else = else_block;
        for (patterns, stmts) in arms.into_iter().rev() {
            let condition = build_when_condition(&subject, patterns);
            let if_stmt = Stmt::If {
                condition,
                then_block: stmts,
                else_block: current_else,
            };
            current_else = Some(vec![if_stmt]);
        }

        let mut result = Vec::new();
        if let Some(stmt) = init_stmt {
            result.push(stmt);
        }
        if let Some(stmts) = current_else {
            result.extend(stmts);
        }
        Ok(result)
    }

    pub(super) fn parse_throw(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'throw'
        if matches!(
            self.current_token().token_type,
            TokenType::Newline
                | TokenType::Eof
                | TokenType::End
                | TokenType::Catch
                | TokenType::Finally
        ) {
            Ok(Stmt::Throw(None))
        } else {
            let expr = self.parse_expression()?;
            Ok(Stmt::Throw(Some(expr)))
        }
    }

    pub(super) fn parse_try_catch(&mut self) -> Result<Stmt, String> {
        let try_line = self.current_token().line;
        let try_col = self.current_token().column;
        self.advance(); // skip 'try'
        self.skip_newlines();

        let mut try_block = Vec::new();
        while !matches!(
            self.current_token().token_type,
            TokenType::Catch | TokenType::Finally | TokenType::End | TokenType::Eof
        ) {
            try_block.extend(self.parse_statement()?);
            self.skip_newlines();
        }

        let mut catch_var = None;
        let mut catch_block = Vec::new();
        let mut has_catch = false;

        if self.current_token().token_type == TokenType::Catch {
            has_catch = true;
            self.advance(); // skip 'catch'

            // Optional catch variable: `catch err`, `catch (err)` or `catch`
            if self.current_token().token_type == TokenType::LeftParen {
                self.advance(); // skip '('
                if let TokenType::Identifier(name) = &self.current_token().token_type {
                    catch_var = Some(name.clone());
                    self.advance();
                } else if self.current_token().token_type == TokenType::RightParen {
                    // empty parens: `catch ()`
                } else {
                    return Err(format!(
                        "Expected identifier inside 'catch (...)' at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ));
                }
                self.expect(TokenType::RightParen)?;
            } else if let TokenType::Identifier(name) = &self.current_token().token_type {
                catch_var = Some(name.clone());
                self.advance();
            }
            self.skip_newlines();

            while !matches!(
                self.current_token().token_type,
                TokenType::Finally | TokenType::End | TokenType::Eof
            ) {
                catch_block.extend(self.parse_statement()?);
                self.skip_newlines();
            }
        }

        let mut finally_block = None;
        if self.current_token().token_type == TokenType::Finally {
            self.advance(); // skip 'finally'
            self.skip_newlines();

            let mut stmts = Vec::new();
            while !matches!(
                self.current_token().token_type,
                TokenType::End | TokenType::Eof
            ) {
                stmts.extend(self.parse_statement()?);
                self.skip_newlines();
            }
            finally_block = Some(stmts);
        }

        if !has_catch && finally_block.is_none() {
            return Err(format!(
                "Expected 'catch' or 'finally' after 'try' block at line {}, column {}",
                try_line, try_col
            ));
        }

        self.expect(TokenType::End)?;

        Ok(Stmt::TryCatch {
            try_block,
            catch_var,
            catch_block,
            finally_block,
        })
    }
}
