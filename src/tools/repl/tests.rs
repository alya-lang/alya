use super::*;
use crate::ast::Stmt;
use crate::codegen::{Architecture, OperatingSystem};
use crate::lexer::Lexer;
use crate::parser::Parser;

#[test]
fn test_completeness_checker() {
    assert!(!is_input_incomplete("1 + 2"));
    assert!(!is_input_incomplete("let x = 10"));
    assert!(!is_input_incomplete("say \"hello\""));

    // Incomplete function
    assert!(is_input_incomplete("function add(a, b)"));
    assert!(!is_input_incomplete("function add(a, b) return a + b end"));

    // Incomplete loop
    assert!(is_input_incomplete("for i in 1..5"));
    assert!(!is_input_incomplete("for i in 1..5 say i end"));

    // Incomplete brackets
    assert!(is_input_incomplete("[1, 2,"));
    assert!(!is_input_incomplete("[1, 2, 3]"));

    // Incomplete operators
    assert!(is_input_incomplete("10 +"));
    assert!(is_input_incomplete("x =="));
}

#[test]
fn test_repl_session_assemble() {
    let mut session = ReplSession::new(Architecture::X64, OperatingSystem::Windows);
    session.imports.push("import \"std/math\"".into());
    session.functions.push((
        "double".into(),
        "function double(x) return x * 2 end".into(),
    ));
    session.statements.push("let x = 10".into());

    let code = session.assemble_program("say double(x)");
    assert!(code.contains("import \"std/math\""));
    assert!(code.contains("function double(x) return x * 2 end"));
    assert!(code.contains("let x = 10"));
    assert!(code.contains("say double(x)"));
}

#[test]
fn test_repl_execute_snippet() {
    let arch = if cfg!(target_arch = "aarch64") {
        Architecture::ARM64
    } else if cfg!(target_arch = "x86") {
        Architecture::X86
    } else {
        Architecture::X64
    };
    let os = if cfg!(target_os = "windows") {
        OperatingSystem::Windows
    } else if cfg!(target_os = "macos") {
        OperatingSystem::MacOS
    } else {
        OperatingSystem::Linux
    };

    let res = execute_code_snippet("say 25 * 4", arch, os);
    if let Ok((success, stdout, _)) = res {
        assert!(success);
        assert_eq!(stdout.trim(), "100");
    }
}

#[test]
fn test_ask_detection_and_freezing() {
    let empty_funcs = Vec::new();

    // 1. Direct ask call in expr
    let mut lexer = Lexer::new("ask \"Name? \"");
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    assert_eq!(ast.statements.len(), 1);
    if let Stmt::Expr(ref expr) = ast.statements[0] {
        assert!(expr_contains_ask(expr, &empty_funcs));
    } else {
        panic!("Expected Stmt::Expr");
    }

    // 2. Pure expr should NOT contain ask
    let mut lexer = Lexer::new("10 + 20 * 3");
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    if let Stmt::Expr(ref expr) = ast.statements[0] {
        assert!(!expr_contains_ask(expr, &empty_funcs));
    } else {
        panic!("Expected Stmt::Expr");
    }

    // 3. Stmt::Let with ask
    let mut lexer = Lexer::new("let name = ask(\"Name? \")");
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    assert!(stmt_contains_ask(&ast.statements[0], &empty_funcs));

    // 4. Stmt::Let with nested ask: int(ask "Age: ")
    let mut lexer = Lexer::new("let age = int(ask \"Age: \")");
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    assert!(stmt_contains_ask(&ast.statements[0], &empty_funcs));

    // 5. Custom function calling ask
    let funcs = vec![(
        "prompt_user".to_string(),
        "function prompt_user() return ask \"Input: \" end".to_string(),
    )];
    let mut lexer = Lexer::new("let x = prompt_user()");
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    assert!(stmt_contains_ask(&ast.statements[0], &funcs));
}

#[test]
fn test_update_or_add_statement() {
    let mut session = ReplSession::new(Architecture::X64, OperatingSystem::Windows);
    session.update_or_add_statement("x", "let x = 10");
    assert_eq!(session.statements.len(), 1);
    assert_eq!(session.statements[0], "let x = 10");

    // Updating x should replace, not append
    session.update_or_add_statement("x", "let x = 20");
    assert_eq!(session.statements.len(), 1);
    assert_eq!(session.statements[0], "let x = 20");

    // Adding x_coord should NOT replace x
    session.update_or_add_statement("x_coord", "let x_coord = 50");
    assert_eq!(session.statements.len(), 2);
    assert_eq!(session.statements[0], "let x = 20");
    assert_eq!(session.statements[1], "let x_coord = 50");
}

#[test]
fn test_completeness_checker_new_features() {
    // Incomplete when
    assert!(is_input_incomplete("when x"));
    assert!(!is_input_incomplete(
        "when x is 1 => \"one\" else => \"other\" end"
    ));

    // Incomplete multiline lambda fn
    assert!(is_input_incomplete("let f = fn(x)"));
    assert!(!is_input_incomplete("let f = fn(x) return x * 2 end"));

    // Incomplete continuation tokens
    assert!(is_input_incomplete("let ok = x in"));
    assert!(is_input_incomplete("let ok = x is"));
    assert!(is_input_incomplete("when x is 1 =>"));
    assert!(is_input_incomplete("1.."));
}

#[test]
fn test_update_or_add_statement_typed_and_pub() {
    let mut session = ReplSession::new(Architecture::X64, OperatingSystem::Windows);
    session.update_or_add_statement("x", "let x: int = 10");
    assert_eq!(session.statements.len(), 1);
    assert_eq!(session.statements[0], "let x: int = 10");

    // Updating typed x should replace, not append
    session.update_or_add_statement("x", "let x: int = 20");
    assert_eq!(session.statements.len(), 1);
    assert_eq!(session.statements[0], "let x: int = 20");

    // Pub let statement
    session.update_or_add_statement("y", "pub let y = 100");
    assert_eq!(session.statements.len(), 2);
    session.update_or_add_statement("y", "pub let y: int = 200");
    assert_eq!(session.statements.len(), 2);
    assert_eq!(session.statements[1], "pub let y: int = 200");
}

#[test]
fn test_repl_execute_new_language_features() {
    let arch = if cfg!(target_arch = "aarch64") {
        Architecture::ARM64
    } else if cfg!(target_arch = "x86") {
        Architecture::X86
    } else {
        Architecture::X64
    };
    let os = if cfg!(target_os = "windows") {
        OperatingSystem::Windows
    } else if cfg!(target_os = "macos") {
        OperatingSystem::MacOS
    } else {
        OperatingSystem::Linux
    };

    let code = r#"
let [a, b] = [10, 20]
let in_test = 20 in [10, 20, 30]
let is_test = "alya" is string
let when_test = when a
    is 10 => "ten"
    else => "other"
end
say str(a + b) + " " + str(in_test) + " " + str(is_test) + " " + when_test
"#;
    let res = execute_code_snippet(code, arch, os);
    if let Ok((success, stdout, _)) = res {
        assert!(success);
        assert_eq!(stdout.trim(), "30 1 1 ten");
    }
}
