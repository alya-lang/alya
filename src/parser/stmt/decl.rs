use crate::ast::*;
use crate::lexer::TokenType;
use crate::parser::Parser;

impl Parser {
    pub(super) fn parse_import(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'import'

        let path = match &self.current_token().token_type {
            TokenType::String(s) => s.clone(),
            _ => {
                return Err(format!(
                    "Expected string literal after 'import' at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ))
            }
        };
        self.advance();

        let alias = if matches!(self.current_token().token_type, TokenType::As) {
            self.advance();
            let alias_name = match &self.current_token().token_type {
                TokenType::Identifier(s) => s.clone(),
                _ => {
                    return Err(format!(
                        "Expected identifier after 'as' at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ))
                }
            };
            self.advance();
            Some(alias_name)
        } else {
            None
        };

        Ok(Stmt::Import {
            path,
            alias,
            symbols: None,
        })
    }

    pub(super) fn parse_from_import(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'from'

        let path = match &self.current_token().token_type {
            TokenType::String(s) => {
                let p = s.clone();
                self.advance();
                p
            }
            TokenType::Identifier(s) => {
                let mut p = s.clone();
                self.advance();
                while matches!(
                    self.current_token().token_type,
                    TokenType::ColonColon | TokenType::Divide
                ) {
                    let sep = if matches!(self.current_token().token_type, TokenType::ColonColon) {
                        "::"
                    } else {
                        "/"
                    };
                    self.advance();
                    match &self.current_token().token_type {
                        TokenType::Identifier(sub) => {
                            p.push_str(sep);
                            p.push_str(sub);
                            self.advance();
                        }
                        _ => {
                            return Err(format!(
                                "Expected identifier in import path at line {}, column {}",
                                self.current_token().line,
                                self.current_token().column
                            ));
                        }
                    }
                }
                p
            }
            _ => {
                return Err(format!(
                    "Expected module path string or identifier after 'from' at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ));
            }
        };

        self.expect(TokenType::Import)?;

        let mut symbols = Vec::new();
        loop {
            let name = match &self.current_token().token_type {
                TokenType::Identifier(s) => {
                    let n = s.clone();
                    self.advance();
                    n
                }
                TokenType::Multiply => {
                    self.advance();
                    "*".to_string()
                }
                _ => {
                    return Err(format!(
                        "Expected symbol name after 'import' at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ));
                }
            };

            let alias = if matches!(self.current_token().token_type, TokenType::As) {
                self.advance();
                let alias_name = match &self.current_token().token_type {
                    TokenType::Identifier(s) => s.clone(),
                    _ => {
                        return Err(format!(
                            "Expected identifier after 'as' at line {}, column {}",
                            self.current_token().line,
                            self.current_token().column
                        ));
                    }
                };
                self.advance();
                Some(alias_name)
            } else {
                None
            };

            symbols.push(ImportSymbol { name, alias });

            if matches!(self.current_token().token_type, TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Stmt::Import {
            path,
            alias: None,
            symbols: Some(symbols),
        })
    }

    pub(super) fn parse_say(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'say'
        let expr = self.parse_expression()?;
        Ok(Stmt::Say(expr))
    }

    pub(super) fn parse_const(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // skip 'const'
        let mut stmts = Vec::new();
        loop {
            let name = match &self.current_token().token_type {
                TokenType::Identifier(s) => s.clone(),
                _ => {
                    return Err(format!(
                        "Expected identifier after 'const' at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ))
                }
            };
            self.advance();
            self.expect(TokenType::Assign)?;
            let value = self.parse_expression()?;
            stmts.push(Stmt::Const { name, value });

            if matches!(self.current_token().token_type, TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(stmts)
    }

    pub(super) fn parse_let(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // skip 'let'

        let first_line = self.current_token().line;
        let first_col = self.current_token().column;

        // 1. Array destructuring: let [a, b, ...rest] = arr
        if matches!(self.current_token().token_type, TokenType::LeftBracket) {
            self.advance(); // skip '['
            let mut names = Vec::new();
            let mut rest_name = None;

            while !matches!(self.current_token().token_type, TokenType::RightBracket) {
                if matches!(
                    self.current_token().token_type,
                    TokenType::DotDotDot | TokenType::DotDot
                ) {
                    self.advance(); // skip '...'
                    let rname = match &self.current_token().token_type {
                        TokenType::Identifier(s) => s.clone(),
                        _ => {
                            return Err(format!(
                                "Expected identifier after '...' at line {}, column {}",
                                self.current_token().line,
                                self.current_token().column
                            ))
                        }
                    };
                    self.advance();
                    rest_name = Some(rname);
                    break;
                }

                let name = match &self.current_token().token_type {
                    TokenType::Identifier(s) => s.clone(),
                    _ => {
                        return Err(format!(
                            "Expected identifier in array destructuring at line {}, column {}",
                            self.current_token().line,
                            self.current_token().column
                        ))
                    }
                };
                self.advance();
                names.push(name);

                if matches!(self.current_token().token_type, TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }

            self.expect(TokenType::RightBracket)?;
            self.expect(TokenType::Assign)?;
            let rhs = self.parse_expression()?;

            let tmp_name = format!("__destruct_arr_{}_{}", first_line, first_col);
            let mut stmts = vec![Stmt::Let {
                name: tmp_name.clone(),
                type_ann: None,
                value: rhs,
            }];

            for (i, name) in names.iter().enumerate() {
                stmts.push(Stmt::Let {
                    name: name.clone(),
                    type_ann: None,
                    value: Expr::Index {
                        array: Box::new(Expr::Identifier(tmp_name.clone())),
                        index: Box::new(Expr::Number(i as f64)),
                    },
                });
            }

            if let Some(rname) = rest_name {
                stmts.push(Stmt::Let {
                    name: rname,
                    type_ann: None,
                    value: Expr::Call {
                        name: "slice".to_string(),
                        args: vec![
                            Expr::Identifier(tmp_name.clone()),
                            Expr::Number(names.len() as f64),
                            Expr::Call {
                                name: "len".to_string(),
                                args: vec![Expr::Identifier(tmp_name.clone())],
                            },
                        ],
                    },
                });
            }

            return Ok(stmts);
        }

        // 2. Map destructuring: let { x, y } = map or let { x: new_x, y } = map
        if matches!(self.current_token().token_type, TokenType::LeftBrace) {
            self.advance(); // skip '{'
            let mut fields = Vec::new();

            while !matches!(self.current_token().token_type, TokenType::RightBrace) {
                let field_key = match &self.current_token().token_type {
                    TokenType::Identifier(s) => s.clone(),
                    TokenType::String(s) => s.clone(),
                    _ => {
                        return Err(format!(
                            "Expected property name in map destructuring at line {}, column {}",
                            self.current_token().line,
                            self.current_token().column
                        ))
                    }
                };
                self.advance();

                let var_name = if matches!(self.current_token().token_type, TokenType::Colon) {
                    self.advance();
                    match &self.current_token().token_type {
                        TokenType::Identifier(s) => {
                            let vname = s.clone();
                            self.advance();
                            vname
                        }
                        _ => {
                            return Err(format!(
                                "Expected identifier after ':' in map destructuring at line {}, column {}",
                                self.current_token().line,
                                self.current_token().column
                            ))
                        }
                    }
                } else {
                    field_key.clone()
                };

                fields.push((field_key, var_name));

                if matches!(self.current_token().token_type, TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }

            self.expect(TokenType::RightBrace)?;
            self.expect(TokenType::Assign)?;
            let rhs = self.parse_expression()?;

            let tmp_name = format!("__destruct_map_{}_{}", first_line, first_col);
            let mut stmts = vec![Stmt::Let {
                name: tmp_name.clone(),
                type_ann: None,
                value: rhs,
            }];

            for (field, var_name) in fields {
                stmts.push(Stmt::Let {
                    name: var_name,
                    type_ann: None,
                    value: Expr::Index {
                        array: Box::new(Expr::Identifier(tmp_name.clone())),
                        index: Box::new(Expr::String(field)),
                    },
                });
            }

            return Ok(stmts);
        }

        let has_parens = if matches!(self.current_token().token_type, TokenType::LeftParen) {
            self.advance();
            true
        } else {
            false
        };

        let mut names = Vec::new();
        let mut types = Vec::new();
        loop {
            let name = match &self.current_token().token_type {
                TokenType::Identifier(s) => s.clone(),
                _ => {
                    return Err(format!(
                        "Expected identifier after 'let' at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ))
                }
            };
            self.advance();

            let type_ann = if matches!(self.current_token().token_type, TokenType::Colon) {
                self.advance();
                let mut t_str = match &self.current_token().token_type {
                    TokenType::Identifier(t) => t.clone(),
                    _ => {
                        return Err(format!(
                            "Expected type after ':' at line {}, column {}",
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

            names.push(name);
            types.push(type_ann);

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

        let first_val = self.parse_expression()?;
        if names.len() == 1
            && matches!(self.current_token().token_type, TokenType::Comma)
            && self.position + 2 < self.tokens.len()
            && matches!(
                self.tokens[self.position + 1].token_type,
                TokenType::Identifier(_)
            )
            && matches!(self.tokens[self.position + 2].token_type, TokenType::Assign)
        {
            let mut stmts = vec![Stmt::Let {
                name: names.remove(0),
                type_ann: types.remove(0),
                value: first_val,
            }];
            while matches!(self.current_token().token_type, TokenType::Comma) {
                self.advance();
                let next_name = match &self.current_token().token_type {
                    TokenType::Identifier(s) => s.clone(),
                    _ => break,
                };
                self.advance();
                let next_type = if matches!(self.current_token().token_type, TokenType::Colon) {
                    self.advance();
                    let mut t_str = match &self.current_token().token_type {
                        TokenType::Identifier(t) => t.clone(),
                        _ => {
                            return Err(format!(
                                "Expected type after ':' at line {}, column {}",
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
                self.expect(TokenType::Assign)?;
                let next_val = self.parse_expression()?;
                stmts.push(Stmt::Let {
                    name: next_name,
                    type_ann: next_type,
                    value: next_val,
                });
            }
            return Ok(stmts);
        }

        let mut values = vec![first_val];
        if matches!(self.current_token().token_type, TokenType::Comma) {
            self.advance();
            loop {
                let value = self.parse_expression()?;
                values.push(value);

                if matches!(self.current_token().token_type, TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        if values.len() == 1 {
            let single_val = values.remove(0);
            if names.len() == 1 {
                Ok(vec![Stmt::Let {
                    name: names.remove(0),
                    type_ann: types.remove(0),
                    value: single_val,
                }])
            } else {
                match &single_val {
                    Expr::Number(_) | Expr::Float(_) | Expr::String(_) | Expr::Null => {
                        let stmts = names
                            .into_iter()
                            .zip(types)
                            .map(|(name, type_ann)| Stmt::Let {
                                name,
                                type_ann,
                                value: single_val.clone(),
                            })
                            .collect();
                        Ok(stmts)
                    }
                    _ => {
                        let tmp_name = format!("__tuple_{}_{}", first_line, first_col);
                        let mut stmts = vec![Stmt::Let {
                            name: tmp_name.clone(),
                            type_ann: None,
                            value: single_val,
                        }];
                        for (i, (name, type_ann)) in names.into_iter().zip(types).enumerate() {
                            stmts.push(Stmt::Let {
                                name,
                                type_ann,
                                value: Expr::Index {
                                    array: Box::new(Expr::Identifier(tmp_name.clone())),
                                    index: Box::new(Expr::Number(i as f64)),
                                },
                            });
                        }
                        Ok(stmts)
                    }
                }
            }
        } else if values.len() == names.len() {
            let has_swap_dependency = names
                .iter()
                .any(|n| values.iter().any(|v| expr_references_name(v, n)));
            if !has_swap_dependency {
                let stmts = names
                    .into_iter()
                    .zip(types)
                    .zip(values)
                    .map(|((name, type_ann), value)| Stmt::Let {
                        name,
                        type_ann,
                        value,
                    })
                    .collect();
                Ok(stmts)
            } else {
                let mut stmts = Vec::new();
                let mut tmp_names = Vec::new();
                for (i, val) in values.into_iter().enumerate() {
                    let tmp_name = format!("__let_tmp_{}_{}_{}", first_line, first_col, i);
                    stmts.push(Stmt::Let {
                        name: tmp_name.clone(),
                        type_ann: None,
                        value: val,
                    });
                    tmp_names.push(tmp_name);
                }
                for ((name, type_ann), tmp_name) in names.into_iter().zip(types).zip(tmp_names) {
                    stmts.push(Stmt::Let {
                        name,
                        type_ann,
                        value: Expr::Identifier(tmp_name),
                    });
                }
                Ok(stmts)
            }
        } else {
            Err(format!(
                "Mismatch in 'let' statement: {} variables defined but {} values provided at line {}, column {}",
                names.len(),
                values.len(),
                first_line,
                first_col
            ))
        }
    }

    pub(super) fn parse_function(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'function'

        let mut name = match &self.current_token().token_type {
            TokenType::Identifier(s) => s.clone(),
            _ => {
                return Err(format!(
                    "Expected function name at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ))
            }
        };
        self.advance();

        while matches!(
            self.current_token().token_type,
            TokenType::ColonColon | TokenType::Dot
        ) {
            self.advance();
            match &self.current_token().token_type {
                TokenType::Identifier(member) => {
                    name = format!("{}__{}", name, member);
                    self.advance();
                }
                _ => {
                    return Err(format!(
                        "Expected identifier after '.' or '::' in function name at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ));
                }
            }
        }

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

            if let TokenType::Identifier(s) = &self.current_token().token_type {
                let param_name = s.clone();
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
                    "Expected parameter name at line {}, column {}",
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

        Ok(Stmt::Function {
            name,
            params,
            param_types,
            return_type,
            defaults,
            body,
        })
    }

    pub(super) fn parse_return(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'return'

        if matches!(
            self.current_token().token_type,
            TokenType::Newline | TokenType::Eof
        ) {
            Ok(Stmt::Return(None))
        } else {
            let expr = self.parse_expression()?;
            if matches!(self.current_token().token_type, TokenType::Comma) {
                let mut exprs = vec![expr];
                while matches!(self.current_token().token_type, TokenType::Comma) {
                    self.advance();
                    self.skip_newlines();
                    exprs.push(self.parse_expression()?);
                }
                Ok(Stmt::Return(Some(Expr::Array(exprs))))
            } else {
                Ok(Stmt::Return(Some(expr)))
            }
        }
    }

    pub(super) fn parse_struct(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'struct'

        let name = match &self.current_token().token_type {
            TokenType::Identifier(s) => s.clone(),
            _ => {
                return Err(format!(
                    "Expected struct name at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ))
            }
        };
        self.advance();
        self.skip_newlines();

        let mut fields = Vec::new();
        let mut field_types = Vec::new();
        let mut defaults = Vec::new();
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
            match &self.current_token().token_type {
                TokenType::Identifier(field) => {
                    fields.push(field.clone());
                    self.advance();

                    let f_type = if matches!(self.current_token().token_type, TokenType::Colon) {
                        self.advance();
                        let mut t_str = match &self.current_token().token_type {
                            TokenType::Identifier(t) => t.clone(),
                            _ => {
                                return Err(format!(
                                    "Expected field type after ':' at line {}, column {}",
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
                    field_types.push(f_type);

                    let default_val =
                        if matches!(self.current_token().token_type, TokenType::Assign) {
                            self.advance();
                            Some(self.parse_expression()?)
                        } else {
                            None
                        };
                    defaults.push(default_val);
                    if matches!(self.current_token().token_type, TokenType::Comma) {
                        self.advance();
                    }
                }
                _ => {
                    return Err(format!(
                        "Expected field name or 'end' at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ))
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::End)?;

        Ok(Stmt::StructDef {
            name,
            fields,
            field_types,
            defaults,
        })
    }

    pub(super) fn parse_enum(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'enum'
        self.skip_newlines();

        let name = match &self.current_token().token_type {
            TokenType::Identifier(s) => s.clone(),
            _ => {
                return Err(format!(
                    "Expected enum name at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ))
            }
        };
        self.advance();
        self.skip_newlines();

        let mut variants = Vec::new();
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
            match &self.current_token().token_type {
                TokenType::Identifier(vname) => {
                    let vname = vname.clone();
                    self.advance();
                    let value = if matches!(self.current_token().token_type, TokenType::Assign) {
                        self.advance();
                        Some(self.parse_expression()?)
                    } else {
                        None
                    };
                    variants.push((vname, value));
                    if matches!(self.current_token().token_type, TokenType::Comma) {
                        self.advance();
                    }
                }
                _ => {
                    return Err(format!(
                        "Expected enum variant or 'end' at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ))
                }
            }
            self.skip_newlines();
        }

        self.expect(TokenType::End)?;

        Ok(Stmt::EnumDef { name, variants })
    }

    pub(super) fn parse_extern(&mut self) -> Result<Stmt, String> {
        self.advance(); // skip 'extern'

        let abi = match &self.current_token().token_type {
            TokenType::String(s) => s.clone(),
            _ => {
                return Err(format!(
                    "Expected ABI string literal after 'extern' (e.g. \"C\") at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ))
            }
        };
        self.advance();

        let lib = if matches!(self.current_token().token_type, TokenType::From) {
            self.advance(); // skip 'from'
            let lib_name = match &self.current_token().token_type {
                TokenType::String(s) => s.clone(),
                _ => {
                    return Err(format!(
                        "Expected library string literal after 'from' at line {}, column {}",
                        self.current_token().line,
                        self.current_token().column
                    ))
                }
            };
            self.advance();
            Some(lib_name)
        } else {
            None
        };

        self.skip_newlines();

        let mut functions = Vec::new();
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

            self.expect(TokenType::Function)?;

            let fn_name = match &self.current_token().token_type {
                TokenType::Identifier(s) => s.clone(),
                _ => {
                    return Err(format!(
                    "Expected function name after 'function' in extern block at line {}, column {}",
                    self.current_token().line,
                    self.current_token().column
                ))
                }
            };
            self.advance();

            self.expect(TokenType::LeftParen)?;

            let mut params = Vec::new();
            while !matches!(
                self.current_token().token_type,
                TokenType::RightParen | TokenType::Eof
            ) {
                let param_name = match &self.current_token().token_type {
                    TokenType::Identifier(s) => s.clone(),
                    _ => {
                        return Err(format!(
                            "Expected parameter name at line {}, column {}",
                            self.current_token().line,
                            self.current_token().column
                        ))
                    }
                };
                self.advance();

                let param_type = if matches!(self.current_token().token_type, TokenType::Colon) {
                    self.advance();
                    match &self.current_token().token_type {
                        TokenType::Identifier(t) => {
                            let t_str = t.clone();
                            self.advance();
                            Some(t_str)
                        }
                        _ => {
                            return Err(format!(
                                "Expected parameter type after ':' at line {}, column {}",
                                self.current_token().line,
                                self.current_token().column
                            ))
                        }
                    }
                } else {
                    None
                };

                params.push(ExternParam {
                    name: param_name,
                    param_type,
                });

                if matches!(self.current_token().token_type, TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }

            self.expect(TokenType::RightParen)?;

            let return_type = if matches!(self.current_token().token_type, TokenType::Arrow) {
                self.advance();
                match &self.current_token().token_type {
                    TokenType::Identifier(t) => {
                        let t_str = t.clone();
                        self.advance();
                        Some(t_str)
                    }
                    _ => {
                        return Err(format!(
                            "Expected return type after '->' at line {}, column {}",
                            self.current_token().line,
                            self.current_token().column
                        ))
                    }
                }
            } else {
                None
            };

            functions.push(ExternFnDecl {
                name: fn_name,
                params,
                return_type,
            });

            self.skip_newlines();
        }

        self.expect(TokenType::End)?;

        Ok(Stmt::ExternBlock {
            abi,
            lib,
            functions,
        })
    }

    pub(crate) fn parse_pub(&mut self) -> Result<Vec<Stmt>, String> {
        self.advance(); // skip 'pub'
        self.skip_newlines();

        let token = self.current_token().clone();
        match &token.token_type {
            TokenType::Function => self.parse_function().map(|s| vec![Stmt::Pub(Box::new(s))]),
            TokenType::Identifier(fn_kw) if fn_kw == "fn" => {
                self.parse_function().map(|s| vec![Stmt::Pub(Box::new(s))])
            }
            TokenType::Struct => self.parse_struct().map(|s| vec![Stmt::Pub(Box::new(s))]),
            TokenType::Enum => self.parse_enum().map(|s| vec![Stmt::Pub(Box::new(s))]),
            TokenType::Const => self
                .parse_const()
                .map(|stmts| stmts.into_iter().map(|s| Stmt::Pub(Box::new(s))).collect()),
            TokenType::Let => self
                .parse_let()
                .map(|stmts| stmts.into_iter().map(|s| Stmt::Pub(Box::new(s))).collect()),
            other => Err(format!(
                "Expected function, struct, enum, const, or let after 'pub' at line {}, column {}, got {:?}",
                token.line, token.column, other
            )),
        }
    }
}

fn expr_references_name(expr: &Expr, name: &str) -> bool {
    match expr {
        Expr::Identifier(s) => s == name,
        Expr::Binary { left, right, .. } => {
            expr_references_name(left, name) || expr_references_name(right, name)
        }
        Expr::Unary { expr, .. } => expr_references_name(expr, name),
        Expr::Call { args, .. } => args.iter().any(|a| expr_references_name(a, name)),
        Expr::Array(items) => items.iter().any(|item| expr_references_name(item, name)),
        Expr::Index { array, index } => {
            expr_references_name(array, name) || expr_references_name(index, name)
        }
        Expr::FieldAccess { object, .. } => expr_references_name(object, name),
        Expr::StructInit { fields, .. } => {
            fields.iter().any(|(_, v)| expr_references_name(v, name))
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_references_name(condition, name)
                || expr_references_name(then_branch, name)
                || expr_references_name(else_branch, name)
        }
        _ => false,
    }
}
