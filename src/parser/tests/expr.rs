use super::*;

#[test]
fn test_parse_binary_precedence() {
    let program = parse_code("say 1 + 2 * 3").expect("Parse failed");
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0] {
        Stmt::Say(Expr::Binary { left, op, right }) => {
            assert_eq!(**left, Expr::Number(1.0));
            assert_eq!(*op, BinaryOp::Add);
            match &**right {
                Expr::Binary {
                    left: rleft,
                    op: rop,
                    right: rright,
                } => {
                    assert_eq!(**rleft, Expr::Number(2.0));
                    assert_eq!(*rop, BinaryOp::Multiply);
                    assert_eq!(**rright, Expr::Number(3.0));
                }
                other => panic!("Expected multiplication on right, got {:?}", other),
            }
        }
        other => panic!("Expected Say with Binary, got {:?}", other),
    }
}

#[test]
fn test_parse_parentheses_precedence() {
    let program = parse_code("say (1 + 2) * 3").expect("Parse failed");

    match &program.statements[0] {
        Stmt::Say(Expr::Binary { left, op, right }) => {
            assert_eq!(*op, BinaryOp::Multiply);
            assert_eq!(**right, Expr::Number(3.0));
            match &**left {
                Expr::Binary {
                    left: lleft,
                    op: lop,
                    right: lright,
                } => {
                    assert_eq!(**lleft, Expr::Number(1.0));
                    assert_eq!(*lop, BinaryOp::Add);
                    assert_eq!(**lright, Expr::Number(2.0));
                }
                other => panic!("Expected addition inside parens, got {:?}", other),
            }
        }
        other => panic!("Expected Say with Binary, got {:?}", other),
    }
}

#[test]
fn test_parse_unary_not_and_negate() {
    let program = parse_code("say -5\nsay not true").expect("Parse failed");
    assert_eq!(program.statements.len(), 2);

    match &program.statements[0] {
        Stmt::Say(Expr::Unary { op, expr }) => {
            assert_eq!(*op, UnaryOp::Negate);
            assert_eq!(**expr, Expr::Number(5.0));
        }
        other => panic!("Expected negate unary, got {:?}", other),
    }

    match &program.statements[1] {
        Stmt::Say(Expr::Unary { op, expr }) => {
            assert_eq!(*op, UnaryOp::Not);
            assert_eq!(**expr, Expr::Number(1.0)); // true is parsed as 1.0
        }
        other => panic!("Expected not unary, got {:?}", other),
    }
}

#[test]
fn test_parse_compound_assignments() {
    let code = "x += 5\ny -= 3\nz *= 2\nw /= 4";
    let program = parse_code(code).expect("Parse failed");
    assert_eq!(program.statements.len(), 4);

    assert_eq!(
        program.statements[0],
        Stmt::Assign {
            name: "x".into(),
            value: Expr::Binary {
                left: Box::new(Expr::Identifier("x".into())),
                op: BinaryOp::Add,
                right: Box::new(Expr::Number(5.0)),
            }
        }
    );

    assert_eq!(
        program.statements[1],
        Stmt::Assign {
            name: "y".into(),
            value: Expr::Binary {
                left: Box::new(Expr::Identifier("y".into())),
                op: BinaryOp::Subtract,
                right: Box::new(Expr::Number(3.0)),
            }
        }
    );

    assert_eq!(
        program.statements[2],
        Stmt::Assign {
            name: "z".into(),
            value: Expr::Binary {
                left: Box::new(Expr::Identifier("z".into())),
                op: BinaryOp::Multiply,
                right: Box::new(Expr::Number(2.0)),
            }
        }
    );

    assert_eq!(
        program.statements[3],
        Stmt::Assign {
            name: "w".into(),
            value: Expr::Binary {
                left: Box::new(Expr::Identifier("w".into())),
                op: BinaryOp::Divide,
                right: Box::new(Expr::Number(4.0)),
            }
        }
    );
}

#[test]
fn test_parse_elif_and_logical_symbols() {
    let code = r#"
if a > 0 && b > 0
    say "both"
elif a > 0 || !c
    say "one or not c"
else
    say "none"
end
"#;
    let program = parse_code(code).expect("Parse failed");
    assert_eq!(program.statements.len(), 1);

    match &program.statements[0] {
        Stmt::If {
            condition,
            then_block: _,
            else_block,
        } => {
            assert_eq!(
                *condition,
                Expr::Binary {
                    left: Box::new(Expr::Binary {
                        left: Box::new(Expr::Identifier("a".into())),
                        op: BinaryOp::Greater,
                        right: Box::new(Expr::Number(0.0)),
                    }),
                    op: BinaryOp::And,
                    right: Box::new(Expr::Binary {
                        left: Box::new(Expr::Identifier("b".into())),
                        op: BinaryOp::Greater,
                        right: Box::new(Expr::Number(0.0)),
                    }),
                }
            );
            assert!(else_block.is_some());
        }
        other => panic!("Expected If, got {:?}", other),
    }
}

#[test]
fn test_parse_ask_expression() {
    let code = "let name = ask \"Your name: \"\nlet city = ask(\"City: \")\nlet general = ask";
    let program = parse_code(code).expect("Parse failed");
    assert_eq!(program.statements.len(), 3);

    assert_eq!(
        program.statements[0],
        Stmt::Let {
            name: "name".into(),
            value: Expr::Call {
                name: "ask".into(),
                args: vec![Expr::String("Your name: ".into())],
            }
        }
    );
}

#[test]
fn test_parse_arrays() {
    let source = "let arr = [1, 2, 3]\nsay arr[0]\narr[1] = 42\nmatrix[0][1] = 99";
    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    assert_eq!(program.statements.len(), 4);

    // let arr = [1, 2, 3]
    match &program.statements[0] {
        Stmt::Let { name, value } => {
            assert_eq!(name, "arr");
            match value {
                Expr::Array(elements) => {
                    assert_eq!(elements.len(), 3);
                    assert_eq!(elements[0], Expr::Number(1.0));
                    assert_eq!(elements[1], Expr::Number(2.0));
                    assert_eq!(elements[2], Expr::Number(3.0));
                }
                other => panic!("Expected Expr::Array, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }

    // say arr[0]
    match &program.statements[1] {
        Stmt::Say(Expr::Index { array, index }) => {
            assert_eq!(**array, Expr::Identifier("arr".into()));
            assert_eq!(**index, Expr::Number(0.0));
        }
        other => panic!("Expected Stmt::Say(Expr::Index), got {:?}", other),
    }

    // arr[1] = 42
    match &program.statements[2] {
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            assert_eq!(*array, Expr::Identifier("arr".into()));
            assert_eq!(*index, Expr::Number(1.0));
            assert_eq!(*value, Expr::Number(42.0));
        }
        other => panic!("Expected Stmt::IndexAssign, got {:?}", other),
    }

    // matrix[0][1] = 99
    match &program.statements[3] {
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            match array {
                Expr::Index {
                    array: inner_array,
                    index: inner_index,
                } => {
                    assert_eq!(**inner_array, Expr::Identifier("matrix".into()));
                    assert_eq!(**inner_index, Expr::Number(0.0));
                }
                other => panic!("Expected Expr::Index, got {:?}", other),
            }
            assert_eq!(*index, Expr::Number(1.0));
            assert_eq!(*value, Expr::Number(99.0));
        }
        other => panic!("Expected Stmt::IndexAssign, got {:?}", other),
    }
}

#[test]
fn test_parse_struct_and_field_access() {
    let source = "struct Point\n  x\n  y\nend\nlet p = Point { x: 10, y: 20 }\np.x = 99\nsay p.x";
    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    assert_eq!(program.statements.len(), 4);

    // 1. StructDef
    match &program.statements[0] {
        Stmt::StructDef { name, fields, .. } => {
            assert_eq!(name, "Point");
            assert_eq!(fields, &["x", "y"]);
        }
        other => panic!("Expected Stmt::StructDef, got {:?}", other),
    }

    // 2. StructInit
    match &program.statements[1] {
        Stmt::Let { name, value } => {
            assert_eq!(name, "p");
            match value {
                Expr::StructInit {
                    name: sname,
                    fields,
                } => {
                    assert_eq!(sname, "Point");
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].0, "x");
                    assert_eq!(fields[1].0, "y");
                }
                other => panic!("Expected Expr::StructInit, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }

    // 3. FieldAssign
    match &program.statements[2] {
        Stmt::FieldAssign {
            object,
            field,
            value,
        } => {
            assert_eq!(*object, Expr::Identifier("p".into()));
            assert_eq!(field, "x");
            assert_eq!(*value, Expr::Number(99.0));
        }
        other => panic!("Expected Stmt::FieldAssign, got {:?}", other),
    }

    // 4. Say with FieldAccess
    match &program.statements[3] {
        Stmt::Say(Expr::FieldAccess { object, field }) => {
            assert_eq!(**object, Expr::Identifier("p".into()));
            assert_eq!(field, "x");
        }
        other => panic!("Expected Stmt::Say(FieldAccess), got {:?}", other),
    }
}

#[test]
fn test_parse_map_literal() {
    let source = "let empty = {}\nlet user = { \"name\": \"Alya\", age: 1, }\nlet explicit = map { \"active\": true }";
    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    assert_eq!(program.statements.len(), 3);

    // 1. empty = {}
    match &program.statements[0] {
        Stmt::Let { name, value } => {
            assert_eq!(name, "empty");
            assert_eq!(*value, Expr::Map(vec![]));
        }
        other => panic!("Expected Stmt::Let with empty map, got {:?}", other),
    }

    // 2. user = { "name": "Alya", age: 1, }
    match &program.statements[1] {
        Stmt::Let { name, value } => {
            assert_eq!(name, "user");
            match value {
                Expr::Map(entries) => {
                    assert_eq!(entries.len(), 2);
                    assert_eq!(
                        entries[0],
                        (Expr::String("name".into()), Expr::String("Alya".into()))
                    );
                    assert_eq!(entries[1], (Expr::String("age".into()), Expr::Number(1.0)));
                }
                other => panic!("Expected Expr::Map, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::Let with map literal, got {:?}", other),
    }

    // 3. explicit = map { "active": true }
    match &program.statements[2] {
        Stmt::Let { name, value } => {
            assert_eq!(name, "explicit");
            match value {
                Expr::Map(entries) => {
                    assert_eq!(entries.len(), 1);
                    assert_eq!(
                        entries[0],
                        (Expr::String("active".into()), Expr::Number(1.0))
                    );
                }
                other => panic!("Expected Expr::Map, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::Let with map literal, got {:?}", other),
    }
}

#[test]
fn test_parse_null_and_nil_literals() {
    let program = parse_code("let x = null\nlet y = nil").expect("Parse failed");
    assert_eq!(program.statements.len(), 2);
    match &program.statements[0] {
        Stmt::Let { name, value } => {
            assert_eq!(name, "x");
            assert_eq!(*value, Expr::Null);
        }
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }
    match &program.statements[1] {
        Stmt::Let { name, value } => {
            assert_eq!(name, "y");
            assert_eq!(*value, Expr::Null);
        }
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }
}

#[test]
fn test_parse_bitwise_operators_and_precedence() {
    // 1 | 2 ^ 3 & 4
    let program = parse_code("say 1 | 2 ^ 3 & 4").expect("Parse failed");
    match &program.statements[0] {
        Stmt::Say(Expr::Binary { left, op, right }) => {
            assert_eq!(*op, BinaryOp::BitOr);
            assert_eq!(**left, Expr::Number(1.0));
            match &**right {
                Expr::Binary {
                    left: xleft,
                    op: xop,
                    right: xright,
                } => {
                    assert_eq!(*xop, BinaryOp::BitXor);
                    assert_eq!(**xleft, Expr::Number(2.0));
                    match &**xright {
                        Expr::Binary {
                            left: aleft,
                            op: aop,
                            right: aright,
                        } => {
                            assert_eq!(*aop, BinaryOp::BitAnd);
                            assert_eq!(**aleft, Expr::Number(3.0));
                            assert_eq!(**aright, Expr::Number(4.0));
                        }
                        other => panic!("Expected BitAnd, got {:?}", other),
                    }
                }
                other => panic!("Expected BitXor, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::Say with BitOr, got {:?}", other),
    }

    // ~a
    let program_not = parse_code("say ~5").expect("Parse failed");
    match &program_not.statements[0] {
        Stmt::Say(Expr::Unary { op, expr }) => {
            assert_eq!(*op, UnaryOp::BitNot);
            assert_eq!(**expr, Expr::Number(5.0));
        }
        other => panic!("Expected Unary BitNot, got {:?}", other),
    }

    // 1 << 2 + 3 -> 1 << (2 + 3)
    let program_shift = parse_code("say 1 << 2 + 3").expect("Parse failed");
    match &program_shift.statements[0] {
        Stmt::Say(Expr::Binary { left, op, right }) => {
            assert_eq!(*op, BinaryOp::Shl);
            assert_eq!(**left, Expr::Number(1.0));
            match &**right {
                Expr::Binary { op: top, .. } => assert_eq!(*top, BinaryOp::Add),
                other => panic!("Expected Add on right of shift, got {:?}", other),
            }
        }
        other => panic!("Expected Shl, got {:?}", other),
    }
}

#[test]
fn test_parse_ternary_and_inline_if() {
    // cond ? a : b
    let program1 = parse_code("say x > 0 ? 1 : 2").expect("Parse failed");
    match &program1.statements[0] {
        Stmt::Say(Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        }) => {
            assert!(matches!(
                **condition,
                Expr::Binary {
                    op: BinaryOp::Greater,
                    ..
                }
            ));
            assert_eq!(**then_branch, Expr::Number(1.0));
            assert_eq!(**else_branch, Expr::Number(2.0));
        }
        other => panic!("Expected Expr::Ternary, got {:?}", other),
    }

    // if cond then a else b
    let program2 = parse_code("say if x > 0 then 1 else 2").expect("Parse failed");
    match &program2.statements[0] {
        Stmt::Say(Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        }) => {
            assert!(matches!(
                **condition,
                Expr::Binary {
                    op: BinaryOp::Greater,
                    ..
                }
            ));
            assert_eq!(**then_branch, Expr::Number(1.0));
            assert_eq!(**else_branch, Expr::Number(2.0));
        }
        other => panic!("Expected Expr::Ternary, got {:?}", other),
    }
}

#[test]
fn test_parse_null_coalesce() {
    let program = parse_code("say port ?? 8080").expect("Parse failed");
    match &program.statements[0] {
        Stmt::Say(Expr::NullCoalesce { value, default }) => {
            assert_eq!(**value, Expr::Identifier("port".into()));
            assert_eq!(**default, Expr::Number(8080.0));
        }
        other => panic!("Expected Expr::NullCoalesce, got {:?}", other),
    }

    // Chained ??
    let program2 = parse_code("say a ?? b ?? 10").expect("Parse failed");
    match &program2.statements[0] {
        Stmt::Say(Expr::NullCoalesce { value, default }) => {
            assert_eq!(**default, Expr::Number(10.0));
            match &**value {
                Expr::NullCoalesce {
                    value: v1,
                    default: d1,
                } => {
                    assert_eq!(**v1, Expr::Identifier("a".into()));
                    assert_eq!(**d1, Expr::Identifier("b".into()));
                }
                other => panic!("Expected inner NullCoalesce, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::NullCoalesce, got {:?}", other),
    }
}

#[test]
fn test_parse_tuple_literal() {
    let program = parse_code("let t = (1, 2, 3)").expect("Parse failed");
    match &program.statements[0] {
        Stmt::Let { name, value } => {
            assert_eq!(name, "t");
            assert_eq!(
                *value,
                Expr::Array(vec![
                    Expr::Number(1.0),
                    Expr::Number(2.0),
                    Expr::Number(3.0),
                ])
            );
        }
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }

    let program_empty = parse_code("let empty = ()").expect("Parse failed");
    match &program_empty.statements[0] {
        Stmt::Let { name, value } => {
            assert_eq!(name, "empty");
            assert_eq!(*value, Expr::Array(vec![]));
        }
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }
}
