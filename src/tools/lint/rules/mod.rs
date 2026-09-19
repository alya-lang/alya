pub mod dead_code;
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

    // Sort diagnostics by line and column
    diagnostics.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.col.cmp(&b.col)));

    diagnostics
}
