use crate::ast::*;
use crate::lexer::TokenType;
use crate::parser::Parser;

#[derive(Debug, Clone)]
pub(crate) enum WhenPattern {
    Exact(Expr),
    Range(Expr, Expr),
    Relational(BinaryOp, Expr),
    Type(String),
    TupleDestructure {
        bindings: Vec<Option<String>>,
        literal_checks: Vec<(usize, Expr)>,
        min_len: usize,
    },
    VariantDestructure {
        type_name: String,
        bindings: Vec<(String, String)>,
    },
}

pub(crate) fn build_pattern_condition(subject: &Expr, pattern: WhenPattern) -> Expr {
    match pattern {
        WhenPattern::Exact(expr) => Expr::Binary {
            left: Box::new(subject.clone()),
            op: BinaryOp::Equal,
            right: Box::new(expr),
        },
        WhenPattern::Type(target) => Expr::TypeCheck {
            expr: Box::new(subject.clone()),
            target,
            negated: false,
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
        WhenPattern::TupleDestructure {
            literal_checks,
            min_len,
            ..
        } => {
            let mut cond = Expr::Binary {
                left: Box::new(Expr::Call {
                    name: "arr_len".to_string(),
                    args: vec![subject.clone()],
                }),
                op: BinaryOp::Equal,
                right: Box::new(Expr::Number(min_len as f64)),
            };
            for (idx, lit) in literal_checks {
                let eq = Expr::Binary {
                    left: Box::new(Expr::Index {
                        array: Box::new(subject.clone()),
                        index: Box::new(Expr::Number(idx as f64)),
                    }),
                    op: BinaryOp::Equal,
                    right: Box::new(lit),
                };
                cond = Expr::Binary {
                    left: Box::new(cond),
                    op: BinaryOp::And,
                    right: Box::new(eq),
                };
            }
            cond
        }
        WhenPattern::VariantDestructure { type_name, .. } => Expr::TypeCheck {
            expr: Box::new(subject.clone()),
            target: type_name,
            negated: false,
        },
    }
}

pub(crate) fn get_pattern_bindings(subject: &Expr, pattern: &WhenPattern) -> Vec<(String, Expr)> {
    match pattern {
        WhenPattern::TupleDestructure { bindings, .. } => {
            let mut res = Vec::new();
            for (i, opt_name) in bindings.iter().enumerate() {
                if let Some(name) = opt_name {
                    res.push((
                        name.clone(),
                        Expr::Index {
                            array: Box::new(subject.clone()),
                            index: Box::new(Expr::Number(i as f64)),
                        },
                    ));
                }
            }
            res
        }
        WhenPattern::VariantDestructure { bindings, .. } => {
            let mut res = Vec::new();
            for (field, var_name) in bindings {
                res.push((
                    var_name.clone(),
                    Expr::FieldAccess {
                        object: Box::new(subject.clone()),
                        field: field.clone(),
                    },
                ));
            }
            res
        }
        _ => Vec::new(),
    }
}

pub(crate) fn substitute_bindings(expr: &Expr, bindings: &[(String, Expr)]) -> Expr {
    if bindings.is_empty() {
        return expr.clone();
    }
    match expr {
        Expr::Identifier(name) => {
            for (var_name, repl) in bindings {
                if name == var_name {
                    return repl.clone();
                }
            }
            Expr::Identifier(name.clone())
        }
        Expr::Binary { left, op, right } => Expr::Binary {
            left: Box::new(substitute_bindings(left, bindings)),
            op: *op,
            right: Box::new(substitute_bindings(right, bindings)),
        },
        Expr::Unary { op, expr: inner } => Expr::Unary {
            op: *op,
            expr: Box::new(substitute_bindings(inner, bindings)),
        },
        Expr::Call { name, args } => Expr::Call {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| substitute_bindings(a, bindings))
                .collect(),
        },
        Expr::InterpolatedString(parts) => Expr::InterpolatedString(
            parts
                .iter()
                .map(|p| substitute_bindings(p, bindings))
                .collect(),
        ),
        Expr::Array(items) => Expr::Array(
            items
                .iter()
                .map(|i| substitute_bindings(i, bindings))
                .collect(),
        ),
        Expr::Index { array, index } => Expr::Index {
            array: Box::new(substitute_bindings(array, bindings)),
            index: Box::new(substitute_bindings(index, bindings)),
        },
        Expr::FieldAccess { object, field } => Expr::FieldAccess {
            object: Box::new(substitute_bindings(object, bindings)),
            field: field.clone(),
        },
        Expr::OptionalFieldAccess { object, field } => Expr::OptionalFieldAccess {
            object: Box::new(substitute_bindings(object, bindings)),
            field: field.clone(),
        },
        Expr::ForceUnwrap(inner) => {
            Expr::ForceUnwrap(Box::new(substitute_bindings(inner, bindings)))
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => Expr::Ternary {
            condition: Box::new(substitute_bindings(condition, bindings)),
            then_branch: Box::new(substitute_bindings(then_branch, bindings)),
            else_branch: Box::new(substitute_bindings(else_branch, bindings)),
        },
        Expr::StructInit { name, fields } => Expr::StructInit {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|(f, v)| (f.clone(), substitute_bindings(v, bindings)))
                .collect(),
        },
        other => other.clone(),
    }
}

pub(crate) fn build_when_condition(subject: &Expr, mut patterns: Vec<WhenPattern>) -> Expr {
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
        if matches!(self.current_token().token_type, TokenType::Then) {
            self.advance();
            let then_block = self.parse_statement()?;
            let else_block = if matches!(self.current_token().token_type, TokenType::Else) {
                self.advance();
                Some(self.parse_statement()?)
            } else {
                None
            };
            return Ok(Stmt::If {
                condition,
                then_block,
                else_block,
            });
        }
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
        let (is_range, inclusive, start, end) = if let Expr::Binary { left, op, right } = &expr {
            if *op == BinaryOp::Range || *op == BinaryOp::RangeInclusive {
                (
                    true,
                    *op == BinaryOp::RangeInclusive,
                    *left.clone(),
                    *right.clone(),
                )
            } else {
                (false, false, expr.clone(), Expr::Null)
            }
        } else if matches!(
            self.current_token().token_type,
            TokenType::DotDot | TokenType::DotDotEqual
        ) {
            let inclusive = matches!(self.current_token().token_type, TokenType::DotDotEqual);
            self.advance();
            let end_expr = self.parse_expression()?;
            (true, inclusive, expr.clone(), end_expr)
        } else {
            (false, false, expr.clone(), Expr::Null)
        };

        if is_range {
            if value_var.is_some() {
                return Err(format!(
                    "Multiple loop variables are not supported for range loops at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ));
            }
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
                start,
                end,
                inclusive,
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
                    type_ann: None,
                    value: raw_subject,
                };
                (Expr::Identifier(temp_name), Some(let_stmt))
            }
        };

        let mut arms: Vec<(Vec<WhenPattern>, Option<Expr>, Vec<Stmt>)> = Vec::new();
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
                        if let Expr::Binary { left, op, right } = pattern_start {
                            if op == BinaryOp::Range || op == BinaryOp::RangeInclusive {
                                patterns.push(WhenPattern::Range(*left, *right));
                            } else {
                                patterns.push(WhenPattern::Exact(Expr::Binary { left, op, right }));
                            }
                        } else if matches!(
                            self.current_token().token_type,
                            TokenType::DotDot | TokenType::DotDotEqual
                        ) {
                            self.advance(); // skip '..' or '..='
                            let pattern_end = self.parse_expression()?;
                            patterns.push(WhenPattern::Range(pattern_start, pattern_end));
                        } else if let Expr::Array(elements) = pattern_start {
                            let mut bindings = Vec::new();
                            let mut literal_checks = Vec::new();
                            let min_len = elements.len();
                            for (i, elem) in elements.into_iter().enumerate() {
                                match elem {
                                    Expr::Identifier(id) => {
                                        if id == "_" {
                                            bindings.push(None);
                                        } else {
                                            bindings.push(Some(id));
                                        }
                                    }
                                    lit => {
                                        bindings.push(None);
                                        literal_checks.push((i, lit));
                                    }
                                }
                            }
                            patterns.push(WhenPattern::TupleDestructure {
                                bindings,
                                literal_checks,
                                min_len,
                            });
                        } else if let Expr::Call { name, args } = pattern_start {
                            if name.chars().next().is_some_and(|c| c.is_uppercase())
                                || name.contains('.')
                            {
                                let bare = name.rsplit("::").next().unwrap_or(&name);
                                let bare = bare.rsplit('.').next().unwrap_or(bare);
                                let known_fields = self
                                    .struct_defs
                                    .get(&name)
                                    .or_else(|| self.struct_defs.get(bare))
                                    .cloned();
                                let mut bindings = Vec::new();
                                for (i, arg) in args.into_iter().enumerate() {
                                    if let Expr::Identifier(var_name) = arg {
                                        if var_name != "_" {
                                            let field_name = if let Some(ref f) = known_fields {
                                                f.get(i).cloned().unwrap_or_else(|| {
                                                    if i == 0 {
                                                        "value".to_string()
                                                    } else {
                                                        format!("f{}", i)
                                                    }
                                                })
                                            } else if bare == "Err" || bare == "Error" {
                                                "message".to_string()
                                            } else if bare == "Ok" || bare == "Some" || i == 0 {
                                                "value".to_string()
                                            } else {
                                                format!("f{}", i)
                                            };
                                            bindings.push((field_name, var_name));
                                        }
                                    }
                                }
                                patterns.push(WhenPattern::VariantDestructure {
                                    type_name: name,
                                    bindings,
                                });
                            } else {
                                patterns.push(WhenPattern::Exact(Expr::Call { name, args }));
                            }
                        } else if let Expr::StructInit { name, fields } = pattern_start {
                            let mut bindings = Vec::new();
                            for (f_name, f_val) in fields {
                                let var_name = match f_val {
                                    Expr::Identifier(v) if v != "_" => v,
                                    _ => f_name.clone(),
                                };
                                bindings.push((f_name, var_name));
                            }
                            patterns.push(WhenPattern::VariantDestructure {
                                type_name: name,
                                bindings,
                            });
                        } else if let Expr::Identifier(ref id) = pattern_start {
                            if id.chars().next().is_some_and(|c| c.is_uppercase())
                                || matches!(
                                    id.as_str(),
                                    "int" | "float" | "string" | "str" | "bool" | "array" | "map"
                                )
                            {
                                patterns.push(WhenPattern::Type(id.clone()));
                            } else {
                                patterns.push(WhenPattern::Exact(pattern_start));
                            }
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

                let guard = if matches!(self.current_token().token_type, TokenType::If) {
                    self.advance();
                    Some(self.parse_expression()?)
                } else {
                    None
                };

                // Optional 'then' or '=>'
                if matches!(
                    self.current_token().token_type,
                    TokenType::Then | TokenType::FatArrow
                ) {
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
                arms.push((patterns, guard, arm_stmts));
            } else if matches!(self.current_token().token_type, TokenType::Else) {
                self.advance(); // skip 'else'
                if matches!(
                    self.current_token().token_type,
                    TokenType::Then | TokenType::FatArrow
                ) {
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
        for (patterns, guard, mut stmts) in arms.into_iter().rev() {
            let mut all_bindings = Vec::new();
            for pat in &patterns {
                all_bindings.extend(get_pattern_bindings(&subject, pat));
            }
            // Prepend bindings for any Destructure or Variant patterns in the arm
            for (var_name, val_expr) in all_bindings.iter().rev() {
                stmts.insert(
                    0,
                    Stmt::Let {
                        name: var_name.clone(),
                        type_ann: None,
                        value: val_expr.clone(),
                    },
                );
            }
            let mut condition = build_when_condition(&subject, patterns);
            if let Some(g) = guard {
                let substituted_guard = substitute_bindings(&g, &all_bindings);
                condition = Expr::Binary {
                    left: Box::new(condition),
                    op: BinaryOp::And,
                    right: Box::new(substituted_guard),
                };
            }
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
