pub mod bool_literals;
pub mod bugs;
pub mod cfg;
pub mod dead_code;
pub mod dynamic;
pub mod naming;
pub mod style;
pub mod unused;

use std::path::Path;

use crate::ast::Program;
use crate::lexer::Token;
use crate::tools::lint::types::LintDiagnostic;

/// Runs all static analysis rules on a parsed program and its token stream.
pub fn run_all_rules(program: &Program, tokens: &[Token], file_path: &Path) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();

    // 1. Unused imports
    diagnostics.extend(unused::check_unused_imports(tokens, file_path));

    // 2. Unused variables
    diagnostics.extend(unused::check_unused_variables(program, tokens, file_path));

    // 3. Unused parameters
    diagnostics.extend(unused::check_unused_parameters(program, tokens, file_path));

    // 4. Dead / unreachable code
    diagnostics.extend(dead_code::check_dead_code(program, tokens, file_path));

    // 5. Idiomatic style & anti-patterns
    diagnostics.extend(style::check_idiomatic_style(program, tokens, file_path));

    // 6. Suspicious bugs (self-comparison, constant-condition, useless-expression)
    diagnostics.extend(bugs::check_suspicious_bugs(program, tokens, file_path));

    // 6b. Bare same-name calls resolving back into the enclosing method
    diagnostics.extend(bugs::check_method_self_recursion(
        program, tokens, file_path,
    ));

    // 6c. Duplicate literal keys in map literals
    diagnostics.extend(bugs::check_duplicate_map_keys(tokens, file_path));

    // 6d. NUL byte inside a string literal (truncates at runtime)
    diagnostics.extend(bugs::check_nul_byte_in_string(tokens, file_path));

    // 7. Naming conventions
    diagnostics.extend(naming::check_naming_conventions(program, tokens, file_path));

    // 8. Unknown @cfg feature names (would silently evaluate to false)
    diagnostics.extend(cfg::check_cfg_features(program, tokens, file_path));

    // 9. `is float` on dynamically-typed values (best-effort at runtime)
    diagnostics.extend(dynamic::check_dynamic_is_float(program, tokens, file_path));

    // 10. Boolean literals and predicate return types
    diagnostics.extend(bool_literals::check_boolean_literals(tokens, file_path));

    // Sort diagnostics by line and column
    diagnostics.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.col.cmp(&b.col)));

    diagnostics
}
