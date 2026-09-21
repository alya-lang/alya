use std::path::Path;

use crate::ast::{Program, Stmt};
use crate::lexer::{Token, TokenType};
use crate::tools::lint::types::{LintDiagnostic, LintSeverity};

fn is_snake_case(s: &str) -> bool {
    let s = s.trim_start_matches('_');
    if s.is_empty() {
        return true;
    }
    // Snake case should not have uppercase characters and should not have double underscores
    !s.chars().any(|c| c.is_uppercase()) && !s.contains("__")
}

fn is_screaming_snake_case(s: &str) -> bool {
    let s = s.trim_start_matches('_');
    if s.is_empty() {
        return true;
    }
    s.chars()
        .all(|c| c.is_uppercase() || c.is_ascii_digit() || c == '_')
}

fn is_pascal_case(s: &str) -> bool {
    let s = s.trim_start_matches('_');
    if s.is_empty() {
        return true;
    }
    // Starts with uppercase, no underscores
    s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && !s.contains('_')
}

fn to_snake_case(s: &str) -> String {
    // If the input is already SCREAMING_SNAKE_CASE (e.g. WORK_IO_READ),
    // just lowercase it: work_io_read — don't insert extra underscores.
    if is_screaming_snake_case(s) {
        return s.to_ascii_lowercase();
    }
    // Otherwise handle camelCase / PascalCase → snake_case conversion:
    // insert '_' before each uppercase letter that follows a lowercase letter or digit.
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() {
            let prev_is_lower =
                i > 0 && (chars[i - 1].is_lowercase() || chars[i - 1].is_ascii_digit());
            let next_is_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
            if i > 0 && !out.ends_with('_') && (prev_is_lower || next_is_lower) {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn to_pascal_case(s: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in s.chars() {
        if ch == '_' {
            capitalize = true;
        } else if capitalize {
            out.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn find_ident_token(tokens: &[Token], ident: &str) -> Option<Token> {
    for tok in tokens {
        if let TokenType::Identifier(ref name) = tok.token_type {
            if name == ident {
                return Some(tok.clone());
            }
        }
    }
    None
}

fn check_stmt_naming(
    stmt: &Stmt,
    tokens: &[Token],
    file_path: &Path,
    diags: &mut Vec<LintDiagnostic>,
) {
    match stmt.inner_stmt() {
        Stmt::Function { name, body, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name);
            if let Some((type_part, method_part)) =
                bare.split_once("__").or_else(|| bare.split_once('.'))
            {
                if !type_part.starts_with('_') && !is_pascal_case(type_part) {
                    let tok = find_ident_token(tokens, type_part);
                    let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                    let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                    diags.push(LintDiagnostic {
                        rule: "naming-convention".to_string(),
                        severity: LintSeverity::Info,
                        message: format!(
                            "struct method receiver type '{}' should follow 'PascalCase' naming convention",
                            type_part
                        ),
                        file_path: file_path.to_path_buf(),
                        line,
                        col,
                        end_line: line,
                        end_col: col + type_part.len(),
                        help: Some(format!("consider renaming to '{}'", to_pascal_case(type_part))),
                        fix: None,
                    });
                }
                if !method_part.starts_with('_') && !is_snake_case(method_part) {
                    let tok = find_ident_token(tokens, method_part);
                    let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                    let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                    diags.push(LintDiagnostic {
                        rule: "naming-convention".to_string(),
                        severity: LintSeverity::Info,
                        message: format!(
                            "method '{}' should follow 'snake_case' naming convention",
                            method_part
                        ),
                        file_path: file_path.to_path_buf(),
                        line,
                        col,
                        end_line: line,
                        end_col: col + method_part.len(),
                        help: Some(format!(
                            "consider renaming to '{}'",
                            to_snake_case(method_part)
                        )),
                        fix: None,
                    });
                }
            } else if !bare.starts_with('_') && !is_snake_case(bare) {
                let tok = find_ident_token(tokens, bare);
                let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                let len = bare.len();
                let suggested = to_snake_case(bare);

                diags.push(LintDiagnostic {
                    rule: "naming-convention".to_string(),
                    severity: LintSeverity::Info,
                    message: format!(
                        "function '{}' should follow 'snake_case' naming convention",
                        bare
                    ),
                    file_path: file_path.to_path_buf(),
                    line,
                    col,
                    end_line: line,
                    end_col: col + len,
                    help: Some(format!("consider renaming to '{}'", suggested)),
                    fix: None,
                });
            }

            for s in body {
                check_stmt_naming(s, tokens, file_path, diags);
            }
        }
        Stmt::StructDef { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name);
            if !bare.starts_with('_') && !is_pascal_case(bare) {
                let tok = find_ident_token(tokens, bare);
                let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                let suggested = to_pascal_case(bare);

                diags.push(LintDiagnostic {
                    rule: "naming-convention".to_string(),
                    severity: LintSeverity::Info,
                    message: format!(
                        "struct '{}' should follow 'PascalCase' naming convention",
                        bare
                    ),
                    file_path: file_path.to_path_buf(),
                    line,
                    col,
                    end_line: line,
                    end_col: col + bare.len(),
                    help: Some(format!("consider renaming to '{}'", suggested)),
                    fix: None,
                });
            }
        }
        Stmt::EnumDef { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name);
            if !bare.starts_with('_') && !is_pascal_case(bare) {
                let tok = find_ident_token(tokens, bare);
                let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                let suggested = to_pascal_case(bare);

                diags.push(LintDiagnostic {
                    rule: "naming-convention".to_string(),
                    severity: LintSeverity::Info,
                    message: format!(
                        "enum '{}' should follow 'PascalCase' naming convention",
                        bare
                    ),
                    file_path: file_path.to_path_buf(),
                    line,
                    col,
                    end_line: line,
                    end_col: col + bare.len(),
                    help: Some(format!("consider renaming to '{}'", suggested)),
                    fix: None,
                });
            }
        }
        Stmt::InterfaceDef { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name);
            if !bare.starts_with('_') && !is_pascal_case(bare) {
                let tok = find_ident_token(tokens, bare);
                let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                let suggested = to_pascal_case(bare);

                diags.push(LintDiagnostic {
                    rule: "naming-convention".to_string(),
                    severity: LintSeverity::Info,
                    message: format!(
                        "interface '{}' should follow 'PascalCase' naming convention",
                        bare
                    ),
                    file_path: file_path.to_path_buf(),
                    line,
                    col,
                    end_line: line,
                    end_col: col + bare.len(),
                    help: Some(format!("consider renaming to '{}'", suggested)),
                    fix: None,
                });
            }
        }
        Stmt::Const { name, .. } => {
            if !name.starts_with('_') && !is_screaming_snake_case(name) && !is_snake_case(name) {
                let tok = find_ident_token(tokens, name);
                let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                let col = tok.as_ref().map(|t| t.column).unwrap_or(1);

                diags.push(LintDiagnostic {
                    rule: "naming-convention".to_string(),
                    severity: LintSeverity::Info,
                    message: format!(
                        "constant '{}' should follow 'SCREAMING_SNAKE_CASE' naming convention",
                        name
                    ),
                    file_path: file_path.to_path_buf(),
                    line,
                    col,
                    end_line: line,
                    end_col: col + name.len(),
                    help: Some(format!("consider renaming to '{}'", name.to_uppercase())),
                    fix: None,
                });
            }
        }
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            for s in then_block {
                check_stmt_naming(s, tokens, file_path, diags);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    check_stmt_naming(s, tokens, file_path, diags);
                }
            }
        }
        Stmt::While { body, .. }
        | Stmt::Repeat { body }
        | Stmt::For { body, .. }
        | Stmt::ForEach { body, .. } => {
            for s in body {
                check_stmt_naming(s, tokens, file_path, diags);
            }
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                check_stmt_naming(s, tokens, file_path, diags);
            }
            for s in catch_block {
                check_stmt_naming(s, tokens, file_path, diags);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    check_stmt_naming(s, tokens, file_path, diags);
                }
            }
        }
        _ => {}
    }
}

/// Checks naming conventions for functions, types, and constants.
pub fn check_naming_conventions(
    program: &Program,
    tokens: &[Token],
    file_path: &Path,
) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();
    for stmt in &program.statements {
        check_stmt_naming(stmt, tokens, file_path, &mut diags);
    }
    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use std::path::PathBuf;

    // ------------------------------------------------------------------ helpers

    fn lint_code(src: &str) -> Vec<LintDiagnostic> {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize().expect("tokenize failed");
        let mut parser = Parser::new(tokens.clone());
        let program = parser.parse().expect("parse failed");
        check_naming_conventions(&program, &tokens, &PathBuf::from("test.alya"))
    }

    fn diag_messages(diags: &[LintDiagnostic]) -> Vec<String> {
        diags.iter().map(|d| d.message.clone()).collect()
    }

    // ------------------------------------------------------------------ is_snake_case

    #[test]
    fn test_is_snake_case_valid() {
        assert!(is_snake_case("foo"));
        assert!(is_snake_case("foo_bar"));
        assert!(is_snake_case("foo_bar_baz"));
        assert!(is_snake_case("_foo"));
        assert!(is_snake_case("__foo")); // leading underscores stripped, then "foo" is valid
        assert!(is_snake_case(""));
        assert!(is_snake_case("a1_b2"));
    }

    #[test]
    fn test_is_snake_case_invalid() {
        assert!(!is_snake_case("FooBar"));
        assert!(!is_snake_case("fooBar"));
        assert!(!is_snake_case("WORK_IO_READ"));
        assert!(!is_snake_case("foo__bar")); // double underscore
    }

    // ------------------------------------------------------------------ is_screaming_snake_case

    #[test]
    fn test_is_screaming_snake_case_valid() {
        assert!(is_screaming_snake_case("WORK_IO_READ"));
        assert!(is_screaming_snake_case("WORK_IO_WRITE"));
        assert!(is_screaming_snake_case("WORK_IO_ACCEPT"));
        assert!(is_screaming_snake_case("WORK_CRYPTO_ENCRYPT"));
        assert!(is_screaming_snake_case("FOO"));
        assert!(is_screaming_snake_case("FOO_BAR_123"));
        assert!(is_screaming_snake_case(""));
    }

    #[test]
    fn test_is_screaming_snake_case_invalid() {
        assert!(!is_screaming_snake_case("fooBar"));
        assert!(!is_screaming_snake_case("foo_bar"));
        assert!(!is_screaming_snake_case("FooBar"));
        assert!(!is_screaming_snake_case("Work_IO"));
    }

    // ------------------------------------------------------------------ is_pascal_case

    #[test]
    fn test_is_pascal_case_valid() {
        assert!(is_pascal_case("FooBar"));
        assert!(is_pascal_case("Foo"));
        assert!(is_pascal_case("MyStruct"));
    }

    #[test]
    fn test_is_pascal_case_invalid() {
        assert!(!is_pascal_case("fooBar"));
        assert!(!is_pascal_case("foo_bar"));
        assert!(!is_pascal_case("FOO_BAR"));
        assert!(!is_pascal_case("Foo_Bar")); // underscore not allowed
    }

    // ------------------------------------------------------------------ to_snake_case

    #[test]
    fn test_to_snake_case_from_pascal() {
        assert_eq!(to_snake_case("FooBar"), "foo_bar");
        assert_eq!(to_snake_case("MyFunc"), "my_func");
        assert_eq!(to_snake_case("Foo"), "foo");
    }

    #[test]
    fn test_to_snake_case_from_camel() {
        assert_eq!(to_snake_case("fooBar"), "foo_bar");
        assert_eq!(to_snake_case("myFuncName"), "my_func_name");
    }

    // Regression: SCREAMING_SNAKE_CASE was producing "w_o_r_k_i_o_r_e_a_d"
    // instead of "work_io_read". Fixed by detecting SCREAMING_SNAKE_CASE and
    // simply lowercasing the entire string.
    #[test]
    fn test_to_snake_case_from_screaming_snake_regression() {
        assert_eq!(to_snake_case("WORK_IO_READ"), "work_io_read");
        assert_eq!(to_snake_case("WORK_IO_WRITE"), "work_io_write");
        assert_eq!(to_snake_case("WORK_IO_ACCEPT"), "work_io_accept");
        assert_eq!(to_snake_case("WORK_CRYPTO_ENCRYPT"), "work_crypto_encrypt");
        assert_eq!(to_snake_case("WORK_CRYPTO_DECRYPT"), "work_crypto_decrypt");
        assert_eq!(to_snake_case("FOO_BAR"), "foo_bar");
        assert_eq!(to_snake_case("FOO"), "foo");
    }

    // ------------------------------------------------------------------ to_pascal_case

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("foo_bar"), "FooBar");
        assert_eq!(to_pascal_case("my_struct"), "MyStruct");
        assert_eq!(to_pascal_case("foo"), "Foo");
    }

    // ------------------------------------------------------------------ function naming diagnostic

    // Regression: SCREAMING_SNAKE_CASE functions (e.g. WORK_IO_READ) used as
    // enum-style constant getters must still emit a snake_case warning, but the
    // suggestion in `help` must be the correctly lowercased form, not a garbled
    // string with underscores between every character.
    #[test]
    fn test_function_screaming_snake_case_suggestion_regression() {
        let diags = lint_code("function WORK_IO_READ() return 3 end");
        assert_eq!(diags.len(), 1, "expected exactly one naming diagnostic");
        let d = &diags[0];
        assert!(
            d.message.contains("WORK_IO_READ"),
            "diagnostic should mention the function name"
        );
        let help = d.help.as_deref().unwrap_or("");
        assert!(
            help.contains("work_io_read"),
            "suggestion should be 'work_io_read', got: {:?}",
            help
        );
        assert!(
            !help.contains("w_o_r_k"),
            "suggestion must not contain garbled 'w_o_r_k' prefix, got: {:?}",
            help
        );
    }

    #[test]
    fn test_function_screaming_snake_case_all_variants() {
        let cases = [
            ("WORK_IO_READ", "work_io_read"),
            ("WORK_IO_WRITE", "work_io_write"),
            ("WORK_IO_ACCEPT", "work_io_accept"),
            ("WORK_CRYPTO_ENCRYPT", "work_crypto_encrypt"),
            ("WORK_CRYPTO_DECRYPT", "work_crypto_decrypt"),
        ];
        for (name, expected_suggestion) in cases {
            let src = format!("function {}() return 1 end", name);
            let diags = lint_code(&src);
            assert_eq!(
                diags.len(),
                1,
                "fn '{}' should produce one diagnostic",
                name
            );
            let help = diags[0].help.as_deref().unwrap_or("");
            assert!(
                help.contains(expected_suggestion),
                "fn '{}': help should suggest '{}', got: {:?}",
                name,
                expected_suggestion,
                help
            );
        }
    }

    #[test]
    fn test_function_snake_case_no_warning() {
        let diags = lint_code("function work_io_read() return 3 end");
        assert!(
            diags.is_empty(),
            "snake_case function should not produce naming warning"
        );
    }

    #[test]
    fn test_function_pascal_case_warning() {
        let diags = lint_code("function MyFunc() return 0 end");
        assert!(!diags.is_empty(), "PascalCase function should warn");
        let help = diags[0].help.as_deref().unwrap_or("");
        assert!(
            help.contains("my_func"),
            "suggestion should be 'my_func', got: {:?}",
            help
        );
    }

    #[test]
    fn test_function_camel_case_warning() {
        let diags = lint_code("function myFunc() return 0 end");
        assert!(!diags.is_empty(), "camelCase function should warn");
        let help = diags[0].help.as_deref().unwrap_or("");
        assert!(
            help.contains("my_func"),
            "suggestion should be 'my_func', got: {:?}",
            help
        );
    }

    // ------------------------------------------------------------------ multiple diagnostics

    #[test]
    fn test_multiple_screaming_functions_each_get_correct_suggestion() {
        let src = r#"
function WORK_IO_READ() return 3 end
function WORK_IO_WRITE() return 4 end
function WORK_IO_ACCEPT() return 5 end
"#;
        let diags = lint_code(src);
        assert_eq!(diags.len(), 3, "expected three diagnostics");
        let helps: Vec<&str> = diags
            .iter()
            .map(|d| d.help.as_deref().unwrap_or(""))
            .collect();
        assert!(helps[0].contains("work_io_read"));
        assert!(helps[1].contains("work_io_write"));
        assert!(helps[2].contains("work_io_accept"));
    }

    // ------------------------------------------------------------------ struct / const naming

    #[test]
    fn test_struct_pascal_case_no_warning() {
        let diags = lint_code("struct MyStruct end");
        assert!(diags.is_empty(), "PascalCase struct should not warn");
    }

    #[test]
    fn test_struct_snake_case_warning() {
        let diags = lint_code("struct my_struct end");
        assert!(!diags.is_empty(), "snake_case struct should warn");
    }

    #[test]
    fn test_const_screaming_snake_case_no_warning() {
        let diags = lint_code("const MAX_SIZE = 1024");
        assert!(
            diags.is_empty(),
            "SCREAMING_SNAKE_CASE const should not warn"
        );
    }

    #[test]
    fn test_const_snake_case_no_warning() {
        let diags = lint_code("const max_size = 1024");
        assert!(diags.is_empty(), "snake_case const should not warn");
    }

    #[test]
    fn test_leading_underscore_function_no_warning() {
        let diags = lint_code("function _internal() return 0 end");
        assert!(
            diag_messages(&diags)
                .iter()
                .all(|m| !m.contains("naming-convention") || !m.contains("_internal")),
            "leading-underscore functions should be exempt from naming warning"
        );
    }
}
