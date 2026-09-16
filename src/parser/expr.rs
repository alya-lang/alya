use super::Parser;
use crate::ast::*;
use crate::lexer::TokenType;
use crate::parser::stmt::control::{build_when_condition, WhenPattern};

impl Parser {
    pub(super) fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Expr, String> {
        if matches!(self.current_token().token_type, TokenType::If) {
            self.advance();
            let condition = self.parse_expression()?;
            self.expect(TokenType::Then)?;
            let then_branch = self.parse_expression()?;
            self.expect(TokenType::Else)?;
            let else_branch = self.parse_expression()?;
            return Ok(Expr::Ternary {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            });
        }

        let expr = self.parse_null_coalesce()?;

        if matches!(self.current_token().token_type, TokenType::Question) {
            self.advance();
            let then_branch = self.parse_expression()?;
            self.expect(TokenType::Colon)?;
            let else_branch = self.parse_expression()?;
            return Ok(Expr::Ternary {
                condition: Box::new(expr),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            });
        }

        Ok(expr)
    }

    fn parse_null_coalesce(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_or()?;

        while matches!(self.current_token().token_type, TokenType::NullCoalesce) {
            self.advance();
            let right = self.parse_or()?;
            left = Expr::NullCoalesce {
                value: Box::new(left),
                default: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;

        while matches!(self.current_token().token_type, TokenType::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_or()?;

        while matches!(self.current_token().token_type, TokenType::And) {
            self.advance();
            let right = self.parse_bitwise_or()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_xor()?;

        while matches!(self.current_token().token_type, TokenType::BitOr) {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::BitOr,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_and()?;

        while matches!(self.current_token().token_type, TokenType::BitXor) {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::BitXor,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;

        while matches!(self.current_token().token_type, TokenType::BitAnd) {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::BitAnd,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_shift()?;

        loop {
            if matches!(self.current_token().token_type, TokenType::In) {
                self.advance();
                let right = self.parse_shift()?;
                left = Expr::Binary {
                    left: Box::new(left),
                    op: BinaryOp::In,
                    right: Box::new(right),
                };
                continue;
            }
            if matches!(self.current_token().token_type, TokenType::Not)
                && self
                    .peek_token()
                    .is_some_and(|t| matches!(t.token_type, TokenType::In))
            {
                self.advance(); // not / !
                self.advance(); // in
                let right = self.parse_shift()?;
                left = Expr::Binary {
                    left: Box::new(left),
                    op: BinaryOp::NotIn,
                    right: Box::new(right),
                };
                continue;
            }
            if matches!(self.current_token().token_type, TokenType::Is) {
                self.advance(); // skip 'is'
                let negated = if matches!(self.current_token().token_type, TokenType::Not) {
                    self.advance(); // skip 'not'
                    true
                } else {
                    false
                };
                let target_type = match &self.current_token().token_type {
                    TokenType::Identifier(id) => id.clone(),
                    TokenType::Null => "null".to_string(),
                    _ => {
                        return Err(format!(
                            "Expected type name after 'is' at line {}, column {}",
                            self.current_token().line,
                            self.current_token().column
                        ));
                    }
                };
                self.advance();
                left = Expr::TypeCheck {
                    expr: Box::new(left),
                    target: target_type,
                    negated,
                };
                continue;
            }

            let op = match &self.current_token().token_type {
                TokenType::Equal => Some(BinaryOp::Equal),
                TokenType::NotEqual => Some(BinaryOp::NotEqual),
                TokenType::Less => Some(BinaryOp::Less),
                TokenType::Greater => Some(BinaryOp::Greater),
                TokenType::LessEqual => Some(BinaryOp::LessEqual),
                TokenType::GreaterEqual => Some(BinaryOp::GreaterEqual),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_shift()?;
                left = Expr::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_term()?;

        while let Some(op) = match &self.current_token().token_type {
            TokenType::Shl => Some(BinaryOp::Shl),
            TokenType::Shr => Some(BinaryOp::Shr),
            _ => None,
        } {
            self.advance();
            let right = self.parse_term()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_factor()?;

        while let Some(op) = match &self.current_token().token_type {
            TokenType::Plus => Some(BinaryOp::Add),
            TokenType::Minus => Some(BinaryOp::Subtract),
            _ => None,
        } {
            self.advance();
            let right = self.parse_factor()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;

        while let Some(op) = match &self.current_token().token_type {
            TokenType::Multiply => Some(BinaryOp::Multiply),
            TokenType::Divide => Some(BinaryOp::Divide),
            TokenType::Modulo => Some(BinaryOp::Modulo),
            _ => None,
        } {
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match &self.current_token().token_type {
            TokenType::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Negate,
                    expr: Box::new(expr),
                })
            }
            TokenType::Not => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            TokenType::BitNot => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::BitNot,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            if matches!(self.current_token().token_type, TokenType::LeftBracket) {
                self.advance();
                if matches!(
                    self.current_token().token_type,
                    TokenType::DotDot | TokenType::Colon
                ) {
                    self.advance();
                    let end = if matches!(self.current_token().token_type, TokenType::RightBracket)
                    {
                        Expr::Number(-1.0)
                    } else {
                        self.parse_expression()?
                    };
                    self.expect(TokenType::RightBracket)?;
                    expr = Expr::Call {
                        name: "slice".into(),
                        args: vec![expr, Expr::Number(0.0), end],
                    };
                } else {
                    let first = self.parse_expression()?;
                    if matches!(
                        self.current_token().token_type,
                        TokenType::DotDot | TokenType::Colon
                    ) {
                        self.advance();
                        let end =
                            if matches!(self.current_token().token_type, TokenType::RightBracket) {
                                Expr::Number(-1.0)
                            } else {
                                self.parse_expression()?
                            };
                        self.expect(TokenType::RightBracket)?;
                        expr = Expr::Call {
                            name: "slice".into(),
                            args: vec![expr, first, end],
                        };
                    } else {
                        self.expect(TokenType::RightBracket)?;
                        expr = Expr::Index {
                            array: Box::new(expr),
                            index: Box::new(first),
                        };
                    }
                }
            } else if matches!(self.current_token().token_type, TokenType::Dot) {
                self.advance();
                let field = match &self.current_token().token_type {
                    TokenType::Identifier(f) => f.clone(),
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
                    let mut args = vec![expr];
                    if !matches!(self.current_token().token_type, TokenType::RightParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.current_token().token_type, TokenType::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(TokenType::RightParen)?;
                    expr = Expr::Call { name: field, args };
                } else {
                    expr = Expr::FieldAccess {
                        object: Box::new(expr),
                        field,
                    };
                }
            } else if matches!(self.current_token().token_type, TokenType::QuestionDot)
                || (matches!(self.current_token().token_type, TokenType::Question)
                    && self
                        .peek_token()
                        .is_some_and(|t| matches!(t.token_type, TokenType::LeftBracket)))
            {
                let is_qdot = matches!(self.current_token().token_type, TokenType::QuestionDot);
                self.advance();
                if !is_qdot || matches!(self.current_token().token_type, TokenType::LeftBracket) {
                    self.advance();
                    if matches!(
                        self.current_token().token_type,
                        TokenType::DotDot | TokenType::Colon
                    ) {
                        self.advance();
                        let end =
                            if matches!(self.current_token().token_type, TokenType::RightBracket) {
                                Expr::Number(-1.0)
                            } else {
                                self.parse_expression()?
                            };
                        self.expect(TokenType::RightBracket)?;
                        expr = Expr::OptionalCall {
                            callee: "slice".into(),
                            args: vec![expr, Expr::Number(0.0), end],
                        };
                    } else {
                        let first = self.parse_expression()?;
                        if matches!(
                            self.current_token().token_type,
                            TokenType::DotDot | TokenType::Colon
                        ) {
                            self.advance();
                            let end = if matches!(
                                self.current_token().token_type,
                                TokenType::RightBracket
                            ) {
                                Expr::Number(-1.0)
                            } else {
                                self.parse_expression()?
                            };
                            self.expect(TokenType::RightBracket)?;
                            expr = Expr::OptionalCall {
                                callee: "slice".into(),
                                args: vec![expr, first, end],
                            };
                        } else {
                            self.expect(TokenType::RightBracket)?;
                            expr = Expr::OptionalIndex {
                                array: Box::new(expr),
                                index: Box::new(first),
                            };
                        }
                    }
                } else if is_qdot && matches!(self.current_token().token_type, TokenType::LeftParen)
                {
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.current_token().token_type, TokenType::RightParen) {
                        args.push(self.parse_expression()?);
                        if matches!(self.current_token().token_type, TokenType::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenType::RightParen)?;
                    let callee = match expr {
                        Expr::Identifier(name) => name,
                        _ => "".to_string(),
                    };
                    expr = Expr::OptionalCall { callee, args };
                } else {
                    let field = match &self.current_token().token_type {
                        TokenType::Identifier(f) => f.clone(),
                        _ => {
                            return Err(format!(
                                "Expected field or method name after '?.' at line {}, column {}",
                                self.current_token().line,
                                self.current_token().column
                            ))
                        }
                    };
                    self.advance();
                    if matches!(self.current_token().token_type, TokenType::LeftParen) {
                        self.advance();
                        let mut args = vec![expr];
                        if !matches!(self.current_token().token_type, TokenType::RightParen) {
                            loop {
                                args.push(self.parse_expression()?);
                                if matches!(self.current_token().token_type, TokenType::Comma) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                        self.expect(TokenType::RightParen)?;
                        expr = Expr::OptionalCall {
                            callee: field,
                            args,
                        };
                    } else {
                        expr = Expr::OptionalFieldAccess {
                            object: Box::new(expr),
                            field,
                        };
                    }
                }
            } else if matches!(self.current_token().token_type, TokenType::LeftParen) {
                if let Expr::Identifier(callee_name) = expr {
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.current_token().token_type, TokenType::RightParen) {
                        args.push(self.parse_expression()?);
                        if matches!(self.current_token().token_type, TokenType::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenType::RightParen)?;
                    expr = Expr::Call {
                        name: callee_name,
                        args,
                    };
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_anonymous_function(&mut self) -> Result<Expr, String> {
        self.expect(TokenType::LeftParen)?;

        let mut params = Vec::new();
        let mut param_types = Vec::new();
        let mut defaults = Vec::new();

        while !matches!(self.current_token().token_type, TokenType::RightParen) {
            let is_rest = if matches!(
                self.current_token().token_type,
                TokenType::DotDotDot | TokenType::DotDot
            ) {
                self.advance();
                true
            } else {
                false
            };

            if let TokenType::Identifier(p) = &self.current_token().token_type {
                let param_name = p.clone();
                self.advance();

                let param_type = if matches!(self.current_token().token_type, TokenType::Colon) {
                    self.advance();
                    let mut t_str = match &self.current_token().token_type {
                        TokenType::Identifier(t) => t.clone(),
                        _ => {
                            return Err(format!(
                                "Expected parameter type after ':' at line {}, column {}",
                                self.current_token().line,
                                self.current_token().column
                            ));
                        }
                    };
                    self.advance();
                    if matches!(self.current_token().token_type, TokenType::LeftBracket) {
                        self.advance();
                        self.expect(TokenType::RightBracket)?;
                        t_str.push_str("[]");
                    }
                    Some(t_str)
                } else if is_rest {
                    Some("...".to_string())
                } else {
                    None
                };

                let default_val = if is_rest {
                    Some(Expr::Array(vec![]))
                } else if matches!(self.current_token().token_type, TokenType::Assign) {
                    self.advance();
                    Some(self.parse_expression()?)
                } else {
                    None
                };

                params.push(param_name);
                param_types.push(param_type);
                defaults.push(default_val);

                if matches!(self.current_token().token_type, TokenType::Comma) {
                    self.advance();
                }
            } else {
                return Err(format!(
                    "Expected parameter name in anonymous function at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ));
            }
        }

        self.expect(TokenType::RightParen)?;

        let return_type = if matches!(self.current_token().token_type, TokenType::Arrow) {
            self.advance();
            let mut t_str = match &self.current_token().token_type {
                TokenType::Identifier(t) => t.clone(),
                _ => {
                    return Err(format!(
                        "Expected return type after '->' at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ));
                }
            };
            self.advance();
            if matches!(self.current_token().token_type, TokenType::LeftBracket) {
                self.advance();
                self.expect(TokenType::RightBracket)?;
                t_str.push_str("[]");
            }
            Some(t_str)
        } else {
            None
        };

        let body = if matches!(self.current_token().token_type, TokenType::FatArrow) {
            self.advance();
            let expr = self.parse_expression()?;
            vec![Stmt::Return(Some(expr))]
        } else {
            self.skip_newlines();
            let mut stmts = Vec::new();
            while !matches!(
                self.current_token().token_type,
                TokenType::End | TokenType::Eof
            ) {
                stmts.extend(self.parse_statement()?);
                self.skip_newlines();
            }
            self.expect(TokenType::End)?;
            stmts
        };

        let lambda_name = format!("__alya_lambda_{}", self.lambda_counter);
        self.lambda_counter += 1;
        self.lambda_functions.push(Stmt::Function {
            name: lambda_name.clone(),
            params,
            param_types,
            return_type,
            defaults,
            body,
        });

        Ok(Expr::Identifier(lambda_name))
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match &self.current_token().token_type {
            TokenType::Number(n) => {
                let num = *n;
                self.advance();
                Ok(Expr::Number(num))
            }
            TokenType::Float(n) => {
                let num = *n;
                self.advance();
                Ok(Expr::Float(num))
            }
            TokenType::String(s) => {
                let string = s.clone();
                self.advance();
                if string.contains('{') && string.contains('}') {
                    if let Some(parts) = parse_interpolated_string(&string) {
                        return Ok(Expr::InterpolatedString(parts));
                    }
                }
                Ok(Expr::String(string))
            }
            TokenType::True => {
                self.advance();
                Ok(Expr::Number(1.0))
            }
            TokenType::False => {
                self.advance();
                Ok(Expr::Number(0.0))
            }
            TokenType::Null => {
                self.advance();
                Ok(Expr::Null)
            }
            TokenType::Function => {
                self.advance();
                self.parse_anonymous_function()
            }
            TokenType::Ask => {
                self.advance();
                let mut args = Vec::new();
                if matches!(self.current_token().token_type, TokenType::LeftParen) {
                    self.advance();
                    if !matches!(self.current_token().token_type, TokenType::RightParen) {
                        args.push(self.parse_expression()?);
                    }
                    self.expect(TokenType::RightParen)?;
                } else if !matches!(
                    self.current_token().token_type,
                    TokenType::Newline
                        | TokenType::Eof
                        | TokenType::RightParen
                        | TokenType::RightBracket
                        | TokenType::RightBrace
                        | TokenType::Comma
                ) {
                    args.push(self.parse_expression()?);
                }
                Ok(Expr::Call {
                    name: "ask".into(),
                    args,
                })
            }
            TokenType::Identifier(name) => {
                let mut ident = name.clone();
                self.advance();

                while matches!(self.current_token().token_type, TokenType::ColonColon) {
                    self.advance();
                    match &self.current_token().token_type {
                        TokenType::Identifier(member) => {
                            ident = format!("{}::{}", ident, member);
                            self.advance();
                        }
                        _ => {
                            return Err(format!(
                                "Expected identifier after '::' at line {}, column {}",
                                self.current_token().line,
                                self.current_token().column
                            ));
                        }
                    }
                }

                // Check for lambda / anonymous function: fn(...) => expr or fn(...) ... end
                if ident == "fn" && matches!(self.current_token().token_type, TokenType::LeftParen)
                {
                    return self.parse_anonymous_function();
                }

                // Check for function call
                if matches!(self.current_token().token_type, TokenType::LeftParen) {
                    self.advance();
                    let mut args = Vec::new();

                    while !matches!(self.current_token().token_type, TokenType::RightParen) {
                        args.push(self.parse_expression()?);
                        if matches!(self.current_token().token_type, TokenType::Comma) {
                            self.advance();
                        }
                    }

                    self.expect(TokenType::RightParen)?;

                    Ok(Expr::Call { name: ident, args })
                } else if ident == "map"
                    && matches!(self.current_token().token_type, TokenType::LeftBrace)
                {
                    self.parse_map_literal()
                } else if matches!(self.current_token().token_type, TokenType::LeftBrace) {
                    self.advance();
                    self.skip_newlines();
                    let mut fields = Vec::new();

                    while !matches!(self.current_token().token_type, TokenType::RightBrace) {
                        self.skip_newlines();
                        if matches!(self.current_token().token_type, TokenType::RightBrace) {
                            break;
                        }
                        let field_name = match &self.current_token().token_type {
                            TokenType::Identifier(f) => f.clone(),
                            _ => {
                                return Err(format!(
                                    "Expected field name in struct initialization at line {}, column {}",
                                    self.current_token().line,
                                    self.current_token().column
                                ))
                            }
                        };
                        self.advance();
                        if matches!(
                            self.current_token().token_type,
                            TokenType::Colon | TokenType::Assign
                        ) {
                            self.advance();
                        } else {
                            return Err(format!(
                                "Expected ':' or '=' after field name at line {}, column {}",
                                self.current_token().line,
                                self.current_token().column
                            ));
                        }
                        let val = self.parse_expression()?;
                        fields.push((field_name, val));
                        self.skip_newlines();
                        if matches!(self.current_token().token_type, TokenType::Comma) {
                            self.advance();
                            self.skip_newlines();
                        }
                    }

                    self.expect(TokenType::RightBrace)?;
                    Ok(Expr::StructInit {
                        name: ident,
                        fields,
                    })
                } else {
                    Ok(Expr::Identifier(ident))
                }
            }
            TokenType::LeftParen => {
                self.advance();
                self.skip_newlines();
                if matches!(self.current_token().token_type, TokenType::RightParen) {
                    self.advance();
                    return Ok(Expr::Array(vec![]));
                }
                let first = self.parse_expression()?;
                self.skip_newlines();
                if matches!(self.current_token().token_type, TokenType::Comma) {
                    let mut elements = vec![first];
                    while matches!(self.current_token().token_type, TokenType::Comma) {
                        self.advance();
                        self.skip_newlines();
                        if matches!(self.current_token().token_type, TokenType::RightParen) {
                            break;
                        }
                        elements.push(self.parse_expression()?);
                        self.skip_newlines();
                    }
                    self.expect(TokenType::RightParen)?;
                    Ok(Expr::Array(elements))
                } else {
                    self.expect(TokenType::RightParen)?;
                    Ok(first)
                }
            }
            TokenType::LeftBrace => self.parse_map_literal(),
            TokenType::LeftBracket => {
                self.advance();
                self.skip_newlines();
                enum ArrayElem {
                    Normal(Expr),
                    Spread(Expr),
                }
                let mut elements = Vec::new();
                let mut has_spread = false;

                while !matches!(self.current_token().token_type, TokenType::RightBracket) {
                    if matches!(
                        self.current_token().token_type,
                        TokenType::DotDotDot | TokenType::DotDot
                    ) {
                        self.advance();
                        has_spread = true;
                        let expr = self.parse_expression()?;
                        elements.push(ArrayElem::Spread(expr));
                    } else {
                        let expr = self.parse_expression()?;
                        elements.push(ArrayElem::Normal(expr));
                    }
                    self.skip_newlines();
                    if matches!(self.current_token().token_type, TokenType::Comma) {
                        self.advance();
                        self.skip_newlines();
                    } else if !matches!(self.current_token().token_type, TokenType::RightBracket) {
                        return Err(format!(
                            "Expected ',' or ']' after array element at line {}, column {}",
                            self.current_token().line,
                            self.current_token().column
                        ));
                    }
                }
                self.expect(TokenType::RightBracket)?;

                if !has_spread {
                    let normal_elems = elements
                        .into_iter()
                        .map(|e| match e {
                            ArrayElem::Normal(ex) => ex,
                            _ => unreachable!(),
                        })
                        .collect();
                    Ok(Expr::Array(normal_elems))
                } else {
                    let lambda_name = format!("__alya_spread_arr_{}", self.lambda_counter);
                    self.lambda_counter += 1;
                    let tmp_res = "__arr_res".to_string();
                    let mut body = vec![Stmt::Let {
                        name: tmp_res.clone(),
                        type_ann: None,
                        value: Expr::Array(vec![]),
                    }];
                    let mut params = Vec::new();
                    let mut param_types = Vec::new();
                    let mut defaults = Vec::new();
                    let mut call_args = Vec::new();

                    for (i, item) in elements.into_iter().enumerate() {
                        let p_name = format!("__arg_{}", i);
                        match item {
                            ArrayElem::Normal(expr) => {
                                call_args.push(expr);
                                params.push(p_name.clone());
                                param_types.push(None);
                                defaults.push(None);
                                body.push(Stmt::Expr(Expr::Call {
                                    name: "push".to_string(),
                                    args: vec![
                                        Expr::Identifier(tmp_res.clone()),
                                        Expr::Identifier(p_name),
                                    ],
                                }));
                            }
                            ArrayElem::Spread(expr) => {
                                call_args.push(expr);
                                params.push(p_name.clone());
                                param_types.push(None);
                                defaults.push(None);
                                let it_var = format!("__item_{}", i);
                                body.push(Stmt::ForEach {
                                    var: it_var.clone(),
                                    value_var: None,
                                    iterable: Expr::Identifier(p_name),
                                    body: vec![Stmt::Expr(Expr::Call {
                                        name: "push".to_string(),
                                        args: vec![
                                            Expr::Identifier(tmp_res.clone()),
                                            Expr::Identifier(it_var),
                                        ],
                                    })],
                                });
                            }
                        }
                    }

                    body.push(Stmt::Return(Some(Expr::Identifier(tmp_res))));

                    self.lambda_functions.push(Stmt::Function {
                        name: lambda_name.clone(),
                        params,
                        param_types,
                        return_type: None,
                        defaults,
                        body,
                    });

                    Ok(Expr::Call {
                        name: lambda_name,
                        args: call_args,
                    })
                }
            }
            TokenType::When => self.parse_when_expression(),
            _ => Err(format!(
                "Unexpected token {} at line {}, column {}",
                self.current_token().token_type,
                self.current_token().line,
                self.current_token().column
            )),
        }
    }

    fn parse_map_literal(&mut self) -> Result<Expr, String> {
        self.expect(TokenType::LeftBrace)?;
        self.skip_newlines();

        enum MapElem {
            Normal(Expr, Expr),
            Spread(Expr),
        }
        let mut entries = Vec::new();
        let mut has_spread = false;

        while !matches!(self.current_token().token_type, TokenType::RightBrace) {
            self.skip_newlines();
            if matches!(self.current_token().token_type, TokenType::RightBrace) {
                break;
            }

            if matches!(
                self.current_token().token_type,
                TokenType::DotDotDot | TokenType::DotDot
            ) {
                self.advance();
                has_spread = true;
                let expr = self.parse_expression()?;
                entries.push(MapElem::Spread(expr));
            } else {
                let key = match &self.current_token().token_type {
                    TokenType::Identifier(id) => {
                        let name = id.clone();
                        self.advance();
                        Expr::String(name)
                    }
                    _ => self.parse_expression()?,
                };
                self.skip_newlines();
                if matches!(
                    self.current_token().token_type,
                    TokenType::Colon | TokenType::Assign
                ) {
                    self.advance();
                } else {
                    return Err(format!(
                        "Expected ':' or '=' after map key at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ));
                }
                self.skip_newlines();
                let val = self.parse_expression()?;
                entries.push(MapElem::Normal(key, val));
            }

            self.skip_newlines();
            if matches!(self.current_token().token_type, TokenType::Comma) {
                self.advance();
                self.skip_newlines();
            } else if !matches!(self.current_token().token_type, TokenType::RightBrace) {
                return Err(format!(
                    "Expected ',' or '}}' after map element at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ));
            }
        }
        self.expect(TokenType::RightBrace)?;

        if !has_spread {
            let normal_entries = entries
                .into_iter()
                .map(|e| match e {
                    MapElem::Normal(k, v) => (k, v),
                    _ => unreachable!(),
                })
                .collect();
            Ok(Expr::Map(normal_entries))
        } else {
            let lambda_name = format!("__alya_spread_map_{}", self.lambda_counter);
            self.lambda_counter += 1;
            let tmp_res = "__map_res".to_string();
            let mut body = vec![Stmt::Let {
                name: tmp_res.clone(),
                type_ann: None,
                value: Expr::Call {
                    name: "map".to_string(),
                    args: vec![],
                },
            }];
            let mut params = Vec::new();
            let mut param_types = Vec::new();
            let mut defaults = Vec::new();
            let mut call_args = Vec::new();

            for (i, item) in entries.into_iter().enumerate() {
                match item {
                    MapElem::Normal(k, v) => {
                        let k_arg = format!("__k_{}", i);
                        let v_arg = format!("__v_{}", i);
                        call_args.push(k);
                        params.push(k_arg.clone());
                        param_types.push(None);
                        defaults.push(None);
                        call_args.push(v);
                        params.push(v_arg.clone());
                        param_types.push(None);
                        defaults.push(None);
                        body.push(Stmt::IndexAssign {
                            array: Expr::Identifier(tmp_res.clone()),
                            index: Expr::Identifier(k_arg),
                            value: Expr::Identifier(v_arg),
                        });
                    }
                    MapElem::Spread(expr) => {
                        let m_arg = format!("__m_{}", i);
                        call_args.push(expr);
                        params.push(m_arg.clone());
                        param_types.push(None);
                        defaults.push(None);
                        let k_var = format!("__k_it_{}", i);
                        let v_var = format!("__v_it_{}", i);
                        body.push(Stmt::ForEach {
                            var: k_var.clone(),
                            value_var: Some(v_var.clone()),
                            iterable: Expr::Identifier(m_arg),
                            body: vec![Stmt::IndexAssign {
                                array: Expr::Identifier(tmp_res.clone()),
                                index: Expr::Identifier(k_var),
                                value: Expr::Identifier(v_var),
                            }],
                        });
                    }
                }
            }

            body.push(Stmt::Return(Some(Expr::Identifier(tmp_res))));

            self.lambda_functions.push(Stmt::Function {
                name: lambda_name.clone(),
                params,
                param_types,
                return_type: None,
                defaults,
                body,
            });

            Ok(Expr::Call {
                name: lambda_name,
                args: call_args,
            })
        }
    }

    pub(super) fn parse_when_expression(&mut self) -> Result<Expr, String> {
        let when_line = self.current_token().line;
        let when_col = self.current_token().column;
        self.advance(); // skip 'when'
        self.skip_newlines();

        let raw_subject = self.parse_expression()?;
        self.skip_newlines();

        let mut arms: Vec<(Vec<WhenPattern>, Expr)> = Vec::new();
        let mut else_expr = None;

        while !matches!(
            self.current_token().token_type,
            TokenType::End | TokenType::Eof
        ) {
            self.skip_newlines();
            if matches!(
                self.current_token().token_type,
                TokenType::End | TokenType::Eof
            ) {
                break;
            }

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

                if matches!(
                    self.current_token().token_type,
                    TokenType::FatArrow | TokenType::Then
                ) {
                    self.advance();
                }
                self.skip_newlines();

                let arm_expr = self.parse_expression()?;
                arms.push((patterns, arm_expr));

                if matches!(self.current_token().token_type, TokenType::Comma) {
                    self.advance();
                }
                self.skip_newlines();
            } else if matches!(self.current_token().token_type, TokenType::Else) {
                self.advance(); // skip 'else'
                if matches!(
                    self.current_token().token_type,
                    TokenType::FatArrow | TokenType::Then
                ) {
                    self.advance();
                }
                self.skip_newlines();
                let e_expr = self.parse_expression()?;
                else_expr = Some(e_expr);

                if matches!(self.current_token().token_type, TokenType::Comma) {
                    self.advance();
                }
                self.skip_newlines();
                break;
            } else {
                return Err(format!(
                    "Expected 'is' or 'else' in 'when' expression at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ));
            }
        }

        self.expect(TokenType::End)?;

        if arms.is_empty() && else_expr.is_none() {
            return Err(format!(
                "Empty 'when' expression at line {}, column {}",
                when_line, when_col
            ));
        }

        let default_else = else_expr.unwrap_or(Expr::Null);
        let mut current_else = default_else;

        for (patterns, expr) in arms.into_iter().rev() {
            let condition = build_when_condition(&raw_subject, patterns);
            current_else = Expr::Ternary {
                condition: Box::new(condition),
                then_branch: Box::new(expr),
                else_branch: Box::new(current_else),
            };
        }

        Ok(current_else)
    }
}

fn parse_interpolated_string(s: &str) -> Option<Vec<Expr>> {
    let mut parts = Vec::new();
    let mut current_lit = String::new();
    let mut chars = s.chars().peekable();
    let mut has_interpolation = false;
    let mut has_escaped = false;

    while let Some(ch) = chars.next() {
        if ch == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                current_lit.push('{');
                has_escaped = true;
                continue;
            }

            let mut expr_str = String::new();
            let mut depth = 1;
            let mut in_str = false;
            let mut in_escape = false;
            let mut found_close = false;

            for c in chars.by_ref() {
                if in_escape {
                    expr_str.push(c);
                    in_escape = false;
                } else if c == '\\' && in_str {
                    expr_str.push(c);
                    in_escape = true;
                } else if c == '"' {
                    in_str = !in_str;
                    expr_str.push(c);
                } else if !in_str && c == '{' {
                    depth += 1;
                    expr_str.push(c);
                } else if !in_str && c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        found_close = true;
                        break;
                    } else {
                        expr_str.push(c);
                    }
                } else {
                    expr_str.push(c);
                }
            }

            let mut parsed_expr = None;
            if found_close && !expr_str.trim().is_empty() {
                let mut sub_lexer = crate::lexer::Lexer::new(&expr_str);
                if let Ok(sub_tokens) = sub_lexer.tokenize() {
                    let mut sub_parser = Parser::new(sub_tokens);
                    if let Ok(expr) = sub_parser.parse_expression() {
                        if sub_parser.current_token().token_type == TokenType::Eof {
                            parsed_expr = Some(expr);
                        }
                    }
                }
            }

            if let Some(expr) = parsed_expr {
                has_interpolation = true;
                if !current_lit.is_empty() {
                    parts.push(Expr::String(current_lit.clone()));
                    current_lit.clear();
                }
                parts.push(expr);
            } else {
                current_lit.push('{');
                current_lit.push_str(&expr_str);
                if found_close {
                    current_lit.push('}');
                }
            }
        } else if ch == '}' && chars.peek() == Some(&'}') {
            chars.next();
            current_lit.push('}');
            has_escaped = true;
        } else {
            current_lit.push(ch);
        }
    }

    if !current_lit.is_empty() {
        parts.push(Expr::String(current_lit));
    }

    if has_interpolation || has_escaped {
        Some(parts)
    } else {
        None
    }
}
