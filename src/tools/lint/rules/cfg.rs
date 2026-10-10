//! `@cfg(feature = ...)` name checking.
//!
//! The compiler evaluates unknown feature names to false (multi-manifest
//! safety), so a typo silently drops code. This rule flags names absent
//! from the enclosing package `[features]` table, closing the loop: every
//! package CI already runs `alya lint --check`.

use std::path::Path;

use crate::ast::Program;
use crate::lexer::{Token, TokenType};
use crate::tools::lint::types::{LintDiagnostic, LintSeverity};
use crate::tools::pkg::discovery::find_manifest_dir_from;
use crate::tools::pkg::manifest::parse_manifest;

fn token_text(token_type: &TokenType) -> Option<&str> {
    match token_type {
        TokenType::Identifier(s) | TokenType::String(s) | TokenType::FormattedString(s) => {
            Some(s.as_str())
        }
        _ => None,
    }
}

pub fn check_cfg_features(
    _program: &Program,
    tokens: &[Token],
    file_path: &Path,
) -> Vec<LintDiagnostic> {
    let base = file_path.parent().unwrap_or(Path::new("."));
    let manifest = (|| {
        let manifest_dir = find_manifest_dir_from(base)?;
        let content = std::fs::read_to_string(manifest_dir.join("alya.toml")).ok()?;
        parse_manifest(&content).ok()
    })();
    let Some(manifest) = manifest else {
        return Vec::new();
    };

    let mut diags = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        // Enter `@cfg(` regions only: a bare `feature = ...` statement
        // elsewhere is user code, not a condition.
        let is_cfg_open = matches!(tokens.get(i).map(|t| &t.token_type), Some(TokenType::At))
            && matches!(
                tokens.get(i + 1).map(|t| &t.token_type),
                Some(TokenType::Identifier(name)) if name == "cfg"
            )
            && matches!(
                tokens.get(i + 2).map(|t| &t.token_type),
                Some(TokenType::LeftParen)
            );
        if !is_cfg_open {
            i += 1;
            continue;
        }
        let mut depth = 0usize;
        let mut j = i + 2;
        while j < tokens.len() {
            match &tokens[j].token_type {
                TokenType::LeftParen => depth += 1,
                TokenType::RightParen => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                TokenType::Identifier(name) if name == "feature" => {
                    let is_assign = matches!(
                        tokens.get(j + 1).map(|t| &t.token_type),
                        Some(TokenType::Assign)
                    );
                    let value = tokens
                        .get(j + 2)
                        .and_then(|t| token_text(&t.token_type).map(|s| s.to_string()));
                    if is_assign {
                        if let Some(feature) = value {
                            if !manifest.features.contains_key(&feature) {
                                let tok = &tokens[j + 2];
                                diags.push(LintDiagnostic {
                                    rule: "cfg-unknown-feature".to_string(),
                                    severity: LintSeverity::Warning,
                                    message: format!(
                                        "Unknown feature '{}' in @cfg: not declared in [features] of alya.toml (evaluates to false)",
                                        feature
                                    ),
                                    file_path: file_path.to_path_buf(),
                                    line: tok.line,
                                    col: tok.column,
                                    end_line: tok.line,
                                    end_col: tok.column + feature.len(),
                                    help: Some(
                                        "declare it under [features] or fix the typo; see `alya help`".to_string(),
                                    ),
                                    fix: None,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
            j += 1;
        }
        i = j + 1;
    }
    diags
}
