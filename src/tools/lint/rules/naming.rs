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

/// Hardware SIMD vector type names (`f32x8`, `i64x4`, `u16x8`, …) mirror ISA
/// and industry spelling (Rust `core::simd`, Intel intrinsics). Forcing
/// PascalCase (`F32x8`) would break platform convention, so these names are
/// exempt from type-naming checks — both as declarations and as method
/// receivers. Pattern: 1+ lowercase letters, 1+ digits, `x`, 1+ digits.
fn is_simd_vector_name(s: &str) -> bool {
    let bytes = s.trim_start_matches('_').as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_lowercase() {
        i += 1;
    }
    let letters = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if letters == 0 || i == letters {
        return false;
    }
    if i >= bytes.len() || bytes[i] != b'x' {
        return false;
    }
    i += 1;
    let lanes_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i == bytes.len() && i > lanes_start
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

/// Locates the definition-site token(s) of `bare` (a `::`-stripped item name)
/// by scanning for the introducing keyword, instead of the first textual
/// occurrence — which may be an unrelated call or use site earlier in the
/// file (e.g. `return time()` on line 16 while `function time__sleep` lives
/// on line 531). Returns the receiver/name token plus, for `Type.method` /
/// `Type::member` definitions (which the parser mangles to `Type__member`
/// in the AST), the method/member token. Returns `None` when the definition
/// pattern is not found so callers can fall back to `find_ident_token`.
fn find_def_site_tokens(
    tokens: &[Token],
    is_keyword: impl Fn(&TokenType) -> bool,
    bare: &str,
) -> Option<(Token, Option<Token>)> {
    let ident_at = |j: usize| -> Option<&str> {
        match tokens.get(j).map(|t| &t.token_type) {
            Some(TokenType::Identifier(n)) => Some(n.as_str()),
            _ => None,
        }
    };
    let is_dot_at = |j: usize| matches!(tokens.get(j).map(|t| &t.token_type), Some(TokenType::Dot));
    let is_coloncolon_at = |j: usize| {
        matches!(
            tokens.get(j).map(|t| &t.token_type),
            Some(TokenType::ColonColon)
        )
    };

    let mut i = 0;
    while i < tokens.len() {
        if is_keyword(&tokens[i].token_type) {
            let mut j = i + 1;
            if let Some(n) = ident_at(j) {
                // Genuine `Type.method` / `Type::member` definition: the
                // parser mangles both separators to `Type__member`.
                let sep_is_member = is_dot_at(j + 1) || is_coloncolon_at(j + 1);
                if sep_is_member {
                    if let Some(m) = ident_at(j + 2) {
                        if format!("{}__{}", n, m) == bare || format!("{}.{}", n, m) == bare {
                            return Some((tokens[j].clone(), Some(tokens[j + 2].clone())));
                        }
                    }
                }
                if n == bare {
                    return Some((tokens[j].clone(), None));
                }
                // Skip `ns::` qualifiers preceding a plain name.
                while ident_at(j).is_some() && is_coloncolon_at(j + 1) {
                    j += 2;
                }
                if let Some(n2) = ident_at(j) {
                    if n2 == bare {
                        return Some((tokens[j].clone(), None));
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Definition-site span of `bare` introduced by `keyword`, falling back to the
/// first textual occurrence and finally to (1, 1).
fn def_span(
    tokens: &[Token],
    is_keyword: impl Fn(&TokenType) -> bool,
    bare: &str,
) -> (usize, usize) {
    find_def_site_tokens(tokens, is_keyword, bare)
        .map(|(tok, _)| (tok.line, tok.column))
        .or_else(|| find_ident_token(tokens, bare).map(|t| (t.line, t.column)))
        .unwrap_or((1, 1))
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
            // Source shape of the definition (tokens carry the ground truth
            // the AST erases): a `Type.method` / `Type::member` definition is
            // mangled to `Type__member` by the parser, exactly like a legacy
            // single-identifier `xx__yy` alias — only the definition-site
            // tokens tell them apart.
            let def_tokens =
                find_def_site_tokens(tokens, |t| matches!(t, TokenType::Function), bare);
            let source_is_method = def_tokens
                .as_ref()
                .is_some_and(|(_, method)| method.is_some());
            // Which naming roles this definition plays: genuine methods (or
            // unknown shapes, preserving legacy behavior) split into
            // receiver + method; single-identifier legacy aliases
            // (e.g. `time__sleep`) are exempt; the rest are plain functions.
            let method_parts: Option<(&str, &str)> = if source_is_method || def_tokens.is_none() {
                bare.split_once("__").or_else(|| bare.split_once('.'))
            } else {
                None
            };
            let is_legacy_alias = !source_is_method && def_tokens.is_some() && bare.contains("__");
            if !is_legacy_alias {
                // Definition-site tokens so diagnostic spans point at the
                // declaration, never at an earlier same-named use.
                if let Some((type_part, method_part)) = method_parts {
                    if !type_part.starts_with('_')
                        && !is_pascal_case(type_part)
                        && !is_simd_vector_name(type_part)
                    {
                        let (line, col) = def_tokens
                            .as_ref()
                            .map(|(recv, _)| (recv.line, recv.column))
                            .or_else(|| {
                                find_ident_token(tokens, type_part).map(|t| (t.line, t.column))
                            })
                            .unwrap_or((1, 1));
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
                        let (line, col) = def_tokens
                            .as_ref()
                            .and_then(|(_, method)| method.as_ref())
                            .map(|t| (t.line, t.column))
                            .or_else(|| {
                                find_ident_token(tokens, method_part).map(|t| (t.line, t.column))
                            })
                            .unwrap_or((1, 1));
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
                    let (line, col, len) = def_tokens
                        .as_ref()
                        .map(|(recv, _)| (recv.line, recv.column, bare.len()))
                        .or_else(|| {
                            find_ident_token(tokens, bare).map(|t| (t.line, t.column, bare.len()))
                        })
                        .unwrap_or((1, 1, bare.len()));
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
            }

            for s in body {
                check_stmt_naming(s, tokens, file_path, diags);
            }
        }
        Stmt::StructDef { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name);
            if !bare.starts_with('_') && !is_pascal_case(bare) && !is_simd_vector_name(bare) {
                let (line, col) = def_span(tokens, |t| matches!(t, TokenType::Struct), bare);
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
            if !bare.starts_with('_') && !is_pascal_case(bare) && !is_simd_vector_name(bare) {
                let (line, col) = def_span(tokens, |t| matches!(t, TokenType::Enum), bare);
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
            if !bare.starts_with('_') && !is_pascal_case(bare) && !is_simd_vector_name(bare) {
                let (line, col) = def_span(tokens, |t| matches!(t, TokenType::Interface), bare);
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
        Stmt::Const { name, .. }
            if !name.starts_with('_') && !is_screaming_snake_case(name) && !is_snake_case(name) =>
        {
            let (line, col) = def_span(tokens, |t| matches!(t, TokenType::Const), name);

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

    // ------------------------------------------------------------------ legacy `__` aliases

    // Regression: `__` marks legacy namespace aliases (e.g. stdlib
    // `time__sleep`), not struct-method receivers. Lint must not demand a
    // PascalCase receiver for them.
    #[test]
    fn test_legacy_double_underscore_function_exempt() {
        let src = r#"
pub function now_millis() -> int
    return clock_ms()
end

pub function time__now_millis() -> int
    return now_millis()
end

pub function time__sleep(ms: int)
    sleep(ms)
end
"#;
        let diags = lint_code(src);
        assert!(
            diags.is_empty(),
            "legacy `__` aliases must not produce naming diagnostics, got: {:?}",
            diag_messages(&diags)
        );
    }

    // ------------------------------------------------------------------ definition-site spans

    // Regression: diagnostic spans pointed at the first textual occurrence
    // of the identifier anywhere in the file (e.g. a `return time()` call on
    // line 16) instead of the offending definition. Spans must resolve to
    // the declaration site.
    #[test]
    fn test_receiver_span_points_at_definition() {
        let src = r#"function probe() -> int
    return mymod.compute(1)
end

pub function mymod.compute(x) -> int
    return x
end
"#;
        let diags = lint_code(src);
        assert_eq!(diags.len(), 1, "expected exactly one naming diagnostic");
        let d = &diags[0];
        assert!(
            d.message.contains("mymod"),
            "diagnostic should mention the receiver, got: {:?}",
            d.message
        );
        assert_eq!(
            d.line, 5,
            "receiver span must point at the definition (line 5), not the use (line 2)"
        );
    }

    // ------------------------------------------------------------------ SIMD vector names

    #[test]
    fn test_struct_span_points_at_definition() {
        let src = r#"function make() -> int
    return widget
end

struct widget
    x: int
end
"#;
        let diags = lint_code(src);
        assert_eq!(diags.len(), 1, "expected exactly one naming diagnostic");
        assert_eq!(
            diags[0].line, 5,
            "struct span must point at the definition (line 5), not the use (line 2)"
        );
    }

    // Hardware vector types (`f32x8`, `i64x4`, …) follow ISA/industry
    // spelling: exempt as declarations and as method receivers, while
    // ordinary lowercase type names must still warn.
    #[test]
    fn test_simd_vector_name_detection() {
        for name in [
            "f32x8", "f64x4", "i32x8", "i64x4", "u16x8", "f16x8", "i8x32",
        ] {
            assert!(
                is_simd_vector_name(name),
                "'{}' should be a SIMD vector name",
                name
            );
        }
        for name in [
            "F32x8", "f32", "f32x", "fx8", "mymod", "widget", "f32x8y", "vec3",
        ] {
            assert!(!is_simd_vector_name(name), "'{}' must NOT match", name);
        }
    }

    #[test]
    fn test_simd_vector_struct_and_receiver_exempt() {
        let src = r#"pub struct f32x8
    handle: ptr
end

pub function f32x8.sum_horizontal(self: f32x8) -> float
    return 1.0
end
"#;
        let diags = lint_code(src);
        assert!(
            diags.is_empty(),
            "SIMD vector struct and receivers must stay silent, got: {:?}",
            diag_messages(&diags)
        );
    }

    #[test]
    fn test_ordinary_lowercase_receiver_still_warns() {
        let src = r#"pub struct widget
    x: int
end

pub function widget.render(self: widget) -> int
    return 1
end
"#;
        let diags = lint_code(src);
        assert_eq!(
            diags.len(),
            2,
            "struct + receiver must both warn, got: {:?}",
            diag_messages(&diags)
        );
        assert!(diags.iter().any(|d| d.message.contains("struct 'widget'")));
        assert!(diags
            .iter()
            .any(|d| d.message.contains("receiver type 'widget'")));
    }
}
