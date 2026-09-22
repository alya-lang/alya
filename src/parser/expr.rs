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
            if matches!(self.current_token().token_type, TokenType::Then) {
                self.advance();
                let then_branch = self.parse_expression()?;
                self.expect(TokenType::Else)?;
                let else_branch = self.parse_expression()?;
                if matches!(self.current_token().token_type, TokenType::End) {
                    self.advance();
                }
                return Ok(Expr::Ternary {
                    condition: Box::new(condition),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                });
            } else {
                self.skip_newlines();
                let then_branch = self.parse_expression()?;
                self.skip_newlines();
                self.expect(TokenType::Else)?;
                self.skip_newlines();
                let else_branch = self.parse_expression()?;
                self.skip_newlines();
                self.expect(TokenType::End)?;
                return Ok(Expr::Ternary {
                    condition: Box::new(condition),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                });
            }
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
        let mut left = self.parse_range()?;

        loop {
            if matches!(self.current_token().token_type, TokenType::As) {
                self.advance();
                let target = self.parse_type_annotation()?;
                left = Expr::Cast {
                    expr: Box::new(left),
                    target,
                };
                continue;
            }
            if matches!(self.current_token().token_type, TokenType::In) {
                self.advance();
                let right = self.parse_range()?;
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
                let right = self.parse_range()?;
                left = Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(Expr::Binary {
                        left: Box::new(left),
                        op: BinaryOp::In,
                        right: Box::new(right),
                    }),
                };
                continue;
            }
            if matches!(self.current_token().token_type, TokenType::Not)
                && self
                    .peek_token()
                    .is_some_and(|t| matches!(t.token_type, TokenType::Is))
            {
                self.advance(); // not / !
                self.advance(); // is
                let target_type = if matches!(self.current_token().token_type, TokenType::Null) {
                    self.advance();
                    "null".to_string()
                } else {
                    self.parse_type_annotation()?
                };
                left = Expr::TypeCheck {
                    expr: Box::new(left),
                    target: target_type,
                    negated: true,
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
                let target_type = if matches!(self.current_token().token_type, TokenType::Null) {
                    self.advance();
                    "null".to_string()
                } else {
                    self.parse_type_annotation()?
                };
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
                let right = self.parse_range()?;
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

    fn parse_range(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_shift()?;

        if matches!(
            self.current_token().token_type,
            TokenType::DotDot | TokenType::DotDotEqual
        ) {
            let op = if matches!(self.current_token().token_type, TokenType::DotDot) {
                BinaryOp::Range
            } else {
                BinaryOp::RangeInclusive
            };
            self.advance();
            let right = if matches!(
                self.current_token().token_type,
                TokenType::RightBracket
                    | TokenType::Comma
                    | TokenType::Newline
                    | TokenType::RightParen
                    | TokenType::RightBrace
                    | TokenType::Eof
            ) {
                Expr::Number(-1.0)
            } else {
                self.parse_shift()?
            };
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
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
            TokenType::Plus => {
                self.advance();
                self.parse_unary()
            }
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
            // Check if there are newlines followed immediately by '.' or '?.'
            let mut peek_idx = self.position;
            while peek_idx < self.tokens.len()
                && matches!(self.tokens[peek_idx].token_type, TokenType::Newline)
            {
                peek_idx += 1;
            }
            if peek_idx > self.position
                && peek_idx < self.tokens.len()
                && matches!(
                    self.tokens[peek_idx].token_type,
                    TokenType::Dot | TokenType::QuestionDot
                )
            {
                self.position = peek_idx;
            }

            if matches!(self.current_token().token_type, TokenType::LeftBracket) {
                self.advance();
                if matches!(
                    self.current_token().token_type,
                    TokenType::DotDot | TokenType::Colon | TokenType::DotDotEqual
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
                    if let Expr::Binary {
                        left: start,
                        op: BinaryOp::Range | BinaryOp::RangeInclusive,
                        right: end,
                    } = first
                    {
                        self.expect(TokenType::RightBracket)?;
                        expr = Expr::Call {
                            name: "slice".into(),
                            args: vec![expr, *start, *end],
                        };
                    } else if matches!(
                        self.current_token().token_type,
                        TokenType::DotDot | TokenType::Colon | TokenType::DotDotEqual
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
                    } else if matches!(self.current_token().token_type, TokenType::Comma) {
                        let mut args = vec![first];
                        while matches!(self.current_token().token_type, TokenType::Comma) {
                            self.advance();
                            args.push(self.parse_expression()?);
                        }
                        self.expect(TokenType::RightBracket)?;
                        expr = Expr::Index {
                            array: Box::new(expr),
                            index: Box::new(Expr::Array(args)),
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
                if let TokenType::Number(n) = self.current_token().token_type {
                    let idx = n as usize;
                    self.advance();
                    expr = Expr::Index {
                        array: Box::new(expr),
                        index: Box::new(Expr::Number(idx as f64)),
                    };
                    continue;
                }
                let field = match &self.current_token().token_type {
                    TokenType::Identifier(f) => f.clone(),
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
                    let mut args = vec![expr];
                    if !matches!(self.current_token().token_type, TokenType::RightParen) {
                        loop {
                            args.push(self.parse_call_argument()?);
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
                        TokenType::DotDot | TokenType::Colon | TokenType::DotDotEqual
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
                        if let Expr::Binary {
                            left: start,
                            op: BinaryOp::Range | BinaryOp::RangeInclusive,
                            right: end,
                        } = first
                        {
                            self.expect(TokenType::RightBracket)?;
                            expr = Expr::OptionalCall {
                                callee: "slice".into(),
                                args: vec![expr, *start, *end],
                            };
                        } else if matches!(
                            self.current_token().token_type,
                            TokenType::DotDot | TokenType::Colon | TokenType::DotDotEqual
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
                        args.push(self.parse_call_argument()?);
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
                                args.push(self.parse_call_argument()?);
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
                        args.push(self.parse_call_argument()?);
                        if matches!(self.current_token().token_type, TokenType::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenType::RightParen)?;
                    expr = Expr::Call {
                        name: callee_name,
                        args,
                    };
                } else if let Expr::Index { array, index } = &expr {
                    let callee_opt = match (&**array, &**index) {
                        (Expr::Identifier(name), Expr::Identifier(type_arg)) => {
                            Some((name.clone(), type_arg.clone()))
                        }
                        (Expr::Identifier(name), Expr::Array(type_args)) => {
                            let parts: Vec<String> = type_args
                                .iter()
                                .filter_map(|e| match e {
                                    Expr::Identifier(id) => Some(id.clone()),
                                    _ => None,
                                })
                                .collect();
                            if parts.len() == type_args.len() && !parts.is_empty() {
                                Some((name.clone(), parts.join("_")))
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    if let Some((callee_name, type_arg)) = callee_opt {
                        self.advance();
                        let mut args = Vec::new();
                        while !matches!(self.current_token().token_type, TokenType::RightParen) {
                            args.push(self.parse_call_argument()?);
                            if matches!(self.current_token().token_type, TokenType::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(TokenType::RightParen)?;
                        expr = Expr::Call {
                            name: format!("{}__{}", callee_name, type_arg),
                            args,
                        };
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else if matches!(self.current_token().token_type, TokenType::LeftBrace) {
                if let Expr::Identifier(name) = &expr {
                    let name = name.clone();
                    self.advance();
                    let fields = self.parse_struct_init_fields()?;
                    expr = Expr::StructInit { name, fields };
                } else if let Expr::Index { array, .. } = &expr {
                    if let Expr::Identifier(name) = &**array {
                        let name = name.clone();
                        self.advance();
                        let fields = self.parse_struct_init_fields()?;
                        expr = Expr::StructInit { name, fields };
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else if matches!(self.current_token().token_type, TokenType::Not) {
                if self
                    .peek_token()
                    .is_some_and(|t| matches!(t.token_type, TokenType::In | TokenType::Is))
                {
                    break;
                }
                self.advance();
                // Postfix ! force unwrap: preserved as ForceUnwrap so the
                // type checker can strip nullability (Chapter 19 §5).
                expr = Expr::ForceUnwrap(Box::new(expr));
            } else {
                break;
            }
        }

        Ok(expr)
    }

    pub(crate) fn parse_call_argument(&mut self) -> Result<Expr, String> {
        if matches!(self.current_token().token_type, TokenType::Identifier(_))
            && self
                .peek_token()
                .is_some_and(|t| t.token_type == TokenType::Colon)
        {
            self.advance(); // consume param name
            self.advance(); // consume ':'
        }
        self.parse_expression()
    }

    fn parse_struct_init_fields(&mut self) -> Result<Vec<(String, Expr)>, String> {
        self.skip_newlines();
        let mut fields = Vec::new();

        while !matches!(
            self.current_token().token_type,
            TokenType::RightBrace | TokenType::Eof
        ) {
            self.skip_newlines();
            if matches!(
                self.current_token().token_type,
                TokenType::RightBrace | TokenType::Eof
            ) {
                break;
            }
            let field_name = match &self.current_token().token_type {
                TokenType::Identifier(f) => f.clone(),
                _ => {
                    return Err(format!(
                        "Expected field name in struct initialization at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ));
                }
            };
            self.advance();
            let val = if matches!(
                self.current_token().token_type,
                TokenType::Colon | TokenType::Assign
            ) {
                self.advance();
                self.parse_expression()?
            } else {
                Expr::Identifier(field_name.clone())
            };
            fields.push((field_name, val));
            self.skip_newlines();
            if matches!(self.current_token().token_type, TokenType::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(TokenType::RightBrace)?;
        Ok(fields)
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

            let param_name = match &self.current_token().token_type {
                TokenType::Identifier(p) => {
                    let n = p.clone();
                    self.advance();
                    n
                }
                TokenType::SelfKw => {
                    self.advance();
                    "self".to_string()
                }
                _ => {
                    return Err(format!(
                        "Expected parameter name in anonymous function at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ));
                }
            };

            let param_type = if matches!(self.current_token().token_type, TokenType::Colon) {
                self.advance();
                Some(self.parse_type_annotation()?)
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
        }

        self.expect(TokenType::RightParen)?;

        let return_type = if matches!(self.current_token().token_type, TokenType::Arrow) {
            self.advance();
            Some(self.parse_type_annotation()?)
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
            type_params: vec![],
        });

        Ok(Expr::Identifier(lambda_name))
    }

    fn parse_closure_body(
        &mut self,
        params: Vec<String>,
        param_types: Vec<Option<String>>,
    ) -> Result<Expr, String> {
        let return_type = if matches!(self.current_token().token_type, TokenType::Arrow) {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let (body, defaults) = if matches!(self.current_token().token_type, TokenType::FatArrow) {
            self.advance();
            if matches!(self.current_token().token_type, TokenType::Newline) {
                self.skip_newlines();
                let mut stmts = Vec::new();
                while !matches!(
                    self.current_token().token_type,
                    TokenType::End | TokenType::RightParen | TokenType::Eof
                ) {
                    stmts.extend(self.parse_statement()?);
                    self.skip_newlines();
                }
                if matches!(self.current_token().token_type, TokenType::End) {
                    self.advance();
                }
                (stmts, vec![None; params.len()])
            } else {
                let expr = self.parse_expression()?;
                (vec![Stmt::Return(Some(expr))], vec![None; params.len()])
            }
        } else {
            self.skip_newlines();
            let mut stmts = Vec::new();
            while !matches!(
                self.current_token().token_type,
                TokenType::End | TokenType::RightParen | TokenType::Eof
            ) {
                stmts.extend(self.parse_statement()?);
                self.skip_newlines();
            }
            if matches!(self.current_token().token_type, TokenType::End) {
                self.advance();
            }
            (stmts, vec![None; params.len()])
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
            type_params: vec![],
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
            TokenType::Rune(c) => {
                let val = *c as u32 as f64;
                self.advance();
                Ok(Expr::Number(val))
            }
            TokenType::SelfKw => {
                self.advance();
                Ok(Expr::Identifier("self".to_string()))
            }
            TokenType::Test => {
                self.advance();
                Ok(Expr::Identifier("test".to_string()))
            }
            TokenType::Assert => {
                self.advance();
                Ok(Expr::Identifier("assert".to_string()))
            }
            TokenType::Bench => {
                self.advance();
                Ok(Expr::Identifier("bench".to_string()))
            }
            TokenType::Comptime => {
                self.advance();
                self.skip_newlines();
                let inner = if matches!(self.current_token().token_type, TokenType::LeftBrace) {
                    self.advance();
                    self.skip_newlines();
                    let mut last = self.parse_expression()?;
                    self.skip_newlines();
                    while !matches!(
                        self.current_token().token_type,
                        TokenType::RightBrace | TokenType::Eof
                    ) {
                        last = self.parse_expression()?;
                        self.skip_newlines();
                    }
                    self.expect(TokenType::RightBrace)?;
                    last
                } else if matches!(self.current_token().token_type, TokenType::LeftParen) {
                    self.advance();
                    self.skip_newlines();
                    let expr = self.parse_expression()?;
                    self.skip_newlines();
                    self.expect(TokenType::RightParen)?;
                    expr
                } else {
                    self.parse_primary()?
                };
                Ok(crate::parser::constants::eval_comptime_expr(&inner))
            }
            TokenType::Sizeof | TokenType::Alignof | TokenType::Typeof => {
                let name = match self.current_token().token_type {
                    TokenType::Sizeof => "sizeof",
                    TokenType::Alignof => "alignof",
                    TokenType::Typeof => "typeof",
                    _ => unreachable!(),
                }
                .to_string();
                self.advance();
                self.expect(TokenType::LeftParen)?;
                let arg = self.parse_expression()?;
                self.expect(TokenType::RightParen)?;
                Ok(Expr::Call {
                    name,
                    args: vec![arg],
                })
            }
            TokenType::Or => {
                self.advance(); // empty params ||
                self.parse_closure_body(vec![], vec![])
            }
            TokenType::BitOr => {
                self.advance(); // consume '|'
                let mut params = Vec::new();
                let mut param_types = Vec::new();
                while !matches!(
                    self.current_token().token_type,
                    TokenType::BitOr | TokenType::Eof
                ) {
                    let name = match &self.current_token().token_type {
                        TokenType::Identifier(id) => id.clone(),
                        TokenType::SelfKw => "self".to_string(),
                        _ => {
                            return Err(format!(
                                "Expected parameter name in closure at line {}, column {}",
                                self.current_token().line,
                                self.current_token().column
                            ));
                        }
                    };
                    self.advance();
                    let p_type = if matches!(self.current_token().token_type, TokenType::Colon) {
                        self.advance();
                        Some(self.parse_type_annotation()?)
                    } else {
                        None
                    };
                    params.push(name);
                    param_types.push(p_type);
                    if matches!(self.current_token().token_type, TokenType::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenType::BitOr)?;
                self.parse_closure_body(params, param_types)
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
                    let member = match &self.current_token().token_type {
                        TokenType::Identifier(member) => member.clone(),
                        tok => {
                            let s = tok.to_string();
                            let clean = s.trim_matches('\'').to_string();
                            if !clean.is_empty()
                                && clean.chars().all(|c| c.is_alphanumeric() || c == '_')
                            {
                                clean
                            } else {
                                return Err(format!(
                                    "Expected identifier after '::' at line {}, column {}",
                                    self.current_token().line,
                                    self.current_token().column
                                ));
                            }
                        }
                    };
                    ident = format!("{}::{}", ident, member);
                    self.advance();
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
                        args.push(self.parse_call_argument()?);
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
                    let fields = self.parse_struct_init_fields()?;
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
                        if matches!(self.current_token().token_type, TokenType::For) {
                            self.advance(); // skip 'for'
                            let var = match &self.current_token().token_type {
                                TokenType::Identifier(id) => id.clone(),
                                _ => {
                                    return Err(format!(
                                        "Expected loop variable after 'for' at line {}, column {}",
                                        self.current_token().line,
                                        self.current_token().column
                                    ))
                                }
                            };
                            self.advance();
                            let value_var =
                                if matches!(self.current_token().token_type, TokenType::Comma) {
                                    self.advance();
                                    match &self.current_token().token_type {
                                        TokenType::Identifier(id) => {
                                            let v = id.clone();
                                            self.advance();
                                            Some(v)
                                        }
                                        _ => {
                                            return Err(
                                                "Expected identifier after comma in comprehension"
                                                    .into(),
                                            )
                                        }
                                    }
                                } else {
                                    None
                                };
                            self.expect(TokenType::In)?;
                            let iterable = self.parse_expression()?;
                            let filter_cond =
                                if matches!(self.current_token().token_type, TokenType::If) {
                                    self.advance();
                                    Some(self.parse_expression()?)
                                } else {
                                    None
                                };
                            self.expect(TokenType::RightBracket)?;

                            let lambda_name = format!("__alya_comp_arr_{}", self.lambda_counter);
                            self.lambda_counter += 1;
                            let tmp_res = "__comp_res".to_string();
                            let mut push_stmt = vec![Stmt::Expr(Expr::Call {
                                name: "push".to_string(),
                                args: vec![Expr::Identifier(tmp_res.clone()), expr],
                            })];
                            if let Some(cond) = filter_cond {
                                push_stmt = vec![Stmt::If {
                                    condition: cond,
                                    then_block: push_stmt,
                                    else_block: None,
                                }];
                            }
                            let body = vec![
                                Stmt::Let {
                                    name: tmp_res.clone(),
                                    type_ann: None,
                                    value: Expr::Array(vec![]),
                                },
                                Stmt::ForEach {
                                    var,
                                    value_var,
                                    iterable,
                                    body: push_stmt,
                                },
                                Stmt::Return(Some(Expr::Identifier(tmp_res))),
                            ];
                            self.lambda_functions.push(Stmt::Function {
                                name: lambda_name.clone(),
                                params: vec![],
                                param_types: vec![],
                                return_type: None,
                                defaults: vec![],
                                body,
                                type_params: vec![],
                            });
                            return Ok(Expr::Call {
                                name: lambda_name,
                                args: vec![],
                            });
                        }
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
                        type_params: vec![],
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
                if matches!(self.current_token().token_type, TokenType::For) {
                    self.advance(); // skip 'for'
                    let var = match &self.current_token().token_type {
                        TokenType::Identifier(id) => id.clone(),
                        _ => {
                            return Err(format!(
                                "Expected loop variable after 'for' at line {}, column {}",
                                self.current_token().line,
                                self.current_token().column
                            ))
                        }
                    };
                    self.advance();
                    let value_var = if matches!(self.current_token().token_type, TokenType::Comma) {
                        self.advance();
                        match &self.current_token().token_type {
                            TokenType::Identifier(id) => {
                                let v = id.clone();
                                self.advance();
                                Some(v)
                            }
                            _ => {
                                return Err(
                                    "Expected identifier after comma in map comprehension".into()
                                )
                            }
                        }
                    } else {
                        None
                    };
                    self.expect(TokenType::In)?;
                    let iterable = self.parse_expression()?;
                    let filter_cond = if matches!(self.current_token().token_type, TokenType::If) {
                        self.advance();
                        Some(self.parse_expression()?)
                    } else {
                        None
                    };
                    self.expect(TokenType::RightBrace)?;

                    let lambda_name = format!("__alya_comp_map_{}", self.lambda_counter);
                    self.lambda_counter += 1;
                    let tmp_res = "__comp_map_res".to_string();
                    let key_expr = match &key {
                        Expr::String(s) => Expr::Identifier(s.clone()),
                        other => other.clone(),
                    };
                    let mut assign_stmt = vec![Stmt::IndexAssign {
                        array: Expr::Identifier(tmp_res.clone()),
                        index: key_expr,
                        value: val,
                    }];
                    if let Some(cond) = filter_cond {
                        assign_stmt = vec![Stmt::If {
                            condition: cond,
                            then_block: assign_stmt,
                            else_block: None,
                        }];
                    }
                    let body = vec![
                        Stmt::Let {
                            name: tmp_res.clone(),
                            type_ann: None,
                            value: Expr::Call {
                                name: "map".to_string(),
                                args: vec![],
                            },
                        },
                        Stmt::ForEach {
                            var,
                            value_var,
                            iterable,
                            body: assign_stmt,
                        },
                        Stmt::Return(Some(Expr::Identifier(tmp_res))),
                    ];
                    self.lambda_functions.push(Stmt::Function {
                        name: lambda_name.clone(),
                        params: vec![],
                        param_types: vec![],
                        return_type: None,
                        defaults: vec![],
                        body,
                        type_params: vec![],
                    });
                    return Ok(Expr::Call {
                        name: lambda_name,
                        args: vec![],
                    });
                }
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
                type_params: vec![],
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

        if matches!(self.current_token().token_type, TokenType::Newline) {
            self.skip_newlines();
            let mut bool_arms = Vec::new();
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
                if matches!(self.current_token().token_type, TokenType::Else) {
                    self.advance();
                    if matches!(
                        self.current_token().token_type,
                        TokenType::FatArrow | TokenType::Then
                    ) {
                        self.advance();
                    }
                    self.skip_newlines();
                    let e_expr = self.parse_expression()?;
                    else_expr = Some(e_expr);
                    self.skip_newlines();
                    break;
                }
                let cond = self.parse_expression()?;
                if matches!(
                    self.current_token().token_type,
                    TokenType::FatArrow | TokenType::Then
                ) {
                    self.advance();
                }
                self.skip_newlines();
                let val = self.parse_expression()?;
                bool_arms.push((cond, val));
                self.skip_newlines();
            }
            self.expect(TokenType::End)?;
            let mut result = else_expr.unwrap_or(Expr::Null);
            for (cond, val) in bool_arms.into_iter().rev() {
                result = Expr::Ternary {
                    condition: Box::new(cond),
                    then_branch: Box::new(val),
                    else_branch: Box::new(result),
                };
            }
            return Ok(result);
        }

        self.skip_newlines();

        let raw_subject = self.parse_expression()?;
        self.skip_newlines();

        let mut arms: Vec<(Vec<WhenPattern>, Option<Expr>, Expr)> = Vec::new();
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

                if matches!(
                    self.current_token().token_type,
                    TokenType::FatArrow | TokenType::Then
                ) {
                    self.advance();
                }
                self.skip_newlines();

                let arm_expr = self.parse_expression()?;
                arms.push((patterns, guard, arm_expr));

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

        for (patterns, guard, mut expr) in arms.into_iter().rev() {
            let mut all_bindings = Vec::new();
            for pat in &patterns {
                let bindings =
                    crate::parser::stmt::control::get_pattern_bindings(&raw_subject, pat);
                all_bindings.extend(bindings);
            }
            expr = crate::parser::stmt::control::substitute_bindings(&expr, &all_bindings);
            let guard =
                guard.map(|g| crate::parser::stmt::control::substitute_bindings(&g, &all_bindings));
            let mut condition = build_when_condition(&raw_subject, patterns);
            if let Some(g) = guard {
                condition = Expr::Binary {
                    left: Box::new(condition),
                    op: BinaryOp::And,
                    right: Box::new(g),
                };
            }
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

            if parsed_expr.is_none() && found_close && !expr_str.trim().is_empty() {
                if let Some(colon_idx) = expr_str.rfind(':') {
                    if colon_idx > 0 && colon_idx < expr_str.len() - 1 {
                        let prev_ch = expr_str.as_bytes()[colon_idx - 1];
                        let next_ch = expr_str.as_bytes()[colon_idx + 1];
                        if prev_ch != b':' && next_ch != b':' {
                            let raw_expr = &expr_str[..colon_idx];
                            let raw_spec = expr_str[colon_idx + 1..].trim();
                            let is_valid_spec = (raw_spec.starts_with('.')
                                && raw_spec.ends_with('f')
                                && raw_spec.len() > 2
                                && raw_spec[1..raw_spec.len() - 1]
                                    .chars()
                                    .all(|c| c.is_ascii_digit()))
                                || (raw_spec.starts_with('0')
                                    && raw_spec.len() > 1
                                    && raw_spec[1..].chars().all(|c| c.is_ascii_digit()))
                                || matches!(
                                    raw_spec,
                                    "x" | "#x" | "X" | "#X" | "b" | "#b" | "d" | "s"
                                )
                                || ((raw_spec.starts_with('>')
                                    || raw_spec.starts_with('<')
                                    || raw_spec.starts_with('^'))
                                    && raw_spec.len() > 1
                                    && raw_spec[1..].chars().all(|c| c.is_ascii_digit()));
                            if is_valid_spec {
                                let mut sub_lexer = crate::lexer::Lexer::new(raw_expr);
                                if let Ok(sub_tokens) = sub_lexer.tokenize() {
                                    let mut sub_parser = Parser::new(sub_tokens);
                                    if let Ok(expr) = sub_parser.parse_expression() {
                                        if sub_parser.current_token().token_type == TokenType::Eof {
                                            parsed_expr = Some(Expr::Call {
                                                name: format!("__alya_format:{}", raw_spec),
                                                args: vec![expr],
                                            });
                                        }
                                    }
                                }
                            }
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
