//! `boolean-literals` rule: prefer `true`/`false` over `1`/`0` in boolean
//! positions, and `-> bool` over `-> int` for predicate functions.
//!
//! Severity is informational on purpose: `alya lint --check` only fails on
//! warnings/errors, so enabling this rule never breaks CI on legacy code,
//! while `alya lint --fix` still applies every finding automatically.
//!
//! Three findings, all with auto-fixes:
//! - `return 0` / `return 1` as the whole return value inside a
//!   predicate-named function (or a `-> bool` one) -> `return false/true`.
//! - `pred(...) == 1` / `!= 0` (and mirrors) where the callee is
//!   predicate-named -> compare against `true`/`false`.
//! - `-> int` on a function whose name is predicate-like and whose every
//!   return is boolean-shaped (literals, comparisons, `not`, `is` checks)
//!   -> `-> bool`.
//!
//! The rule is token-based (the parser desugars `true`/`false` to `1`/`0`,
//! so the AST cannot tell them apart) with best-effort block tracking. If
//! nesting ever becomes inconsistent the rule goes silent for the rest of
//! the file rather than risk a wrong fix.

use std::path::Path;

use crate::lexer::{Token, TokenType};
use crate::tools::lint::types::{LintDiagnostic, LintFix, LintSeverity};

/// A bare function name counts as predicate-like when it reads as a
/// yes/no question. `count_*` / `len*` / `index*` holders are excluded on
/// purpose (`count_matches` returns a count, not a verdict).
fn is_predicate_bare(name: &str) -> bool {
    if name.starts_with("is_")
        || name.starts_with("has_")
        || name.starts_with("can_")
        || name.starts_with("should_")
    {
        return true;
    }
    let lowered = name.to_lowercase();
    let hits = lowered.contains("matches")
        || lowered.contains("valid")
        || lowered.contains("exists")
        || lowered.contains("verify")
        || lowered.contains("equal")
        || name == "eq"
        || name.ends_with("_eq");
    if !hits {
        return false;
    }
    !(lowered.contains("count") || lowered.contains("len") || lowered.contains("index"))
}

/// Last identifier segment of a possibly qualified name
/// (`Regex.is_match` -> `is_match`, `re::is_match` -> `is_match`).
fn bare_name(name: &str) -> &str {
    let after_dot = name.rsplit('.').next().unwrap_or(name);
    after_dot.rsplit("::").next().unwrap_or(after_dot)
}

fn is_bool_annotation(ann: &Option<String>) -> bool {
    matches!(ann.as_deref(), Some("bool") | Some("boolean"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Bool,
    Other,
}

struct FnFrame {
    bare: String,
    ret_ann: Option<String>,
    /// Span of the `int` in `-> int`, for the annotation fix.
    ann_span: Option<(usize, usize)>,
    /// Block depth the function body started at.
    depth_base: usize,
    saw_return: bool,
    all_bool: bool,
}

struct Scanner<'a> {
    tokens: &'a [Token],
    diags: Vec<LintDiagnostic>,
    file_path: &'a Path,
}

struct HeaderInfo {
    name: String,
    ret_ann: Option<String>,
    ann_span: Option<(usize, usize)>,
    body_start: usize,
}

impl<'a> Scanner<'a> {
    fn peek(&self, idx: usize) -> Option<&Token> {
        self.tokens.get(idx)
    }

    fn diag(
        &mut self,
        message: String,
        line: usize,
        col: usize,
        end_col: usize,
        help: &str,
        replacement: &str,
    ) {
        self.diags.push(LintDiagnostic {
            rule: "boolean-literals".to_string(),
            severity: LintSeverity::Info,
            message,
            file_path: self.file_path.to_path_buf(),
            line,
            col,
            end_line: line,
            end_col,
            help: Some(help.to_string()),
            fix: Some(LintFix {
                description: format!("replace with '{replacement}'"),
                replacement: replacement.to_string(),
                start_line: line,
                start_col: col,
                end_line: line,
                end_col,
            }),
        });
    }

    /// Parse `function Name[.method][[T]](params) [-> Ret]` starting at the
    /// `function` keyword.
    fn parse_header(&self, start: usize) -> Option<HeaderInfo> {
        let mut i = start + 1;
        let mut name = String::new();
        loop {
            match self.peek(i).map(|t| &t.token_type) {
                Some(TokenType::Identifier(s)) => {
                    if !name.is_empty() {
                        name.push('.');
                    }
                    name.push_str(s);
                    i += 1;
                }
                Some(TokenType::Dot) => {
                    i += 1;
                    continue;
                }
                _ => break,
            }
            if !matches!(self.peek(i).map(|t| &t.token_type), Some(TokenType::Dot)) {
                break;
            }
        }
        if name.is_empty() {
            return None;
        }
        // Optional generic parameter list `[T, ...]`.
        if matches!(
            self.peek(i).map(|t| &t.token_type),
            Some(TokenType::LeftBracket)
        ) {
            let mut depth = 0usize;
            while let Some(tok) = self.peek(i) {
                match tok.token_type {
                    TokenType::LeftBracket => depth += 1,
                    TokenType::RightBracket => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    TokenType::Newline | TokenType::End => return None,
                    _ => {}
                }
                i += 1;
            }
        }
        // Parameter list `( ... )`.
        if !matches!(
            self.peek(i).map(|t| &t.token_type),
            Some(TokenType::LeftParen)
        ) {
            return None;
        }
        let mut depth = 0usize;
        while let Some(tok) = self.peek(i) {
            match tok.token_type {
                TokenType::LeftParen => depth += 1,
                TokenType::RightParen => {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                TokenType::Newline => {
                    // Multi-line signatures are out of scope.
                    return None;
                }
                _ => {}
            }
            i += 1;
        }
        // Optional `-> Ret`.
        let mut ann = None;
        let mut ann_span = None;
        if matches!(self.peek(i).map(|t| &t.token_type), Some(TokenType::Arrow)) {
            i += 1;
            if let Some(tok) = self.peek(i) {
                if let TokenType::Identifier(t) = &tok.token_type {
                    // Only plain `int` gets a fix span (`int[]` stays manual).
                    if t == "int"
                        && !matches!(
                            self.peek(i + 1).map(|x| &x.token_type),
                            Some(TokenType::LeftBracket)
                        )
                    {
                        ann_span = Some((tok.line, tok.column));
                    }
                    ann = Some(t.clone());
                    i += 1;
                }
            }
        }
        Some(HeaderInfo {
            name,
            ret_ann: ann,
            ann_span,
            body_start: i,
        })
    }

    /// Classify a `return <expr>` value shape from tokens starting just
    /// after `return`. Stops at a newline, `end`, or a top-level comma
    /// (tuple returns like `return 0, -1` are never plain booleans).
    /// Returns (shape, index of first token past the value).
    fn return_shape(&self, start: usize) -> (Shape, usize) {
        let mut i = start;
        let mut has_cmp = false;
        let mut has_not = false;
        let mut bool_lit = false;
        let mut other = false;
        let mut depth = 0usize;
        let mut first = true;
        while let Some(tok) = self.peek(i) {
            match &tok.token_type {
                TokenType::Newline => break,
                TokenType::End if depth == 0 => break,
                TokenType::Comma if depth == 0 => {
                    other = true;
                    break;
                }
                TokenType::LeftParen | TokenType::LeftBracket => {
                    depth += 1;
                }
                TokenType::RightParen | TokenType::RightBracket => {
                    depth = depth.saturating_sub(1);
                }
                TokenType::Number(n) if first && (*n == 0 || *n == 1) => {
                    bool_lit = true;
                }
                TokenType::True | TokenType::False if first => {
                    bool_lit = true;
                }
                TokenType::Equal
                | TokenType::NotEqual
                | TokenType::Less
                | TokenType::Greater
                | TokenType::LessEqual
                | TokenType::GreaterEqual
                | TokenType::Is => {
                    if depth == 0 {
                        has_cmp = true;
                    }
                }
                TokenType::Not if first => {
                    has_not = true;
                }
                TokenType::When
                | TokenType::FatArrow
                | TokenType::LeftBrace
                | TokenType::Plus
                | TokenType::Minus
                | TokenType::Multiply
                | TokenType::Divide
                | TokenType::Modulo
                | TokenType::And
                | TokenType::Or
                | TokenType::String(_)
                | TokenType::Float(_)
                | TokenType::Null => {
                    other = true;
                }
                _ => {}
            }
            first = false;
            i += 1;
        }
        let shape = if other || (!bool_lit && !has_cmp && !has_not) {
            Shape::Other
        } else {
            Shape::Bool
        };
        (shape, i)
    }

    /// Bare `Number(0|1)` immediately after `return`, terminated by a
    /// newline or `end` (so `return 0, -1` and `return 0 + x` are left
    /// alone). Returns the literal token index on success.
    fn bare_literal_after_return(&self, start: usize) -> Option<usize> {
        let lit = self.peek(start)?;
        if !matches!(lit.token_type, TokenType::Number(0) | TokenType::Number(1)) {
            return None;
        }
        match self.peek(start + 1).map(|t| &t.token_type) {
            Some(TokenType::Newline) | Some(TokenType::End) | None => Some(start),
            _ => None,
        }
    }

    /// `pred(...) == 1` / `!= 0` (and mirrors): when the callee's bare
    /// name is predicate-like, the literal is a boolean spelling.
    fn check_call_comparison(&mut self, op_idx: usize) {
        let lit = match self.peek(op_idx + 1) {
            Some(t) => t,
            None => return,
        };
        let word = match lit.token_type {
            TokenType::Number(0) => "false",
            TokenType::Number(1) => "true",
            _ => return,
        };
        // Left side must end with `)` (a call result, not `1 == 1`).
        let i = match op_idx.checked_sub(1) {
            Some(i) => i,
            None => return,
        };
        if !matches!(
            self.peek(i).map(|t| &t.token_type),
            Some(TokenType::RightParen)
        ) {
            return;
        }
        // Walk back over balanced parens to the matching `(`.
        let mut depth = 1usize;
        let mut j = i;
        let open = loop {
            if j == 0 {
                return;
            }
            j -= 1;
            match self.peek(j).map(|t| &t.token_type) {
                Some(TokenType::RightParen) => depth += 1,
                Some(TokenType::LeftParen) => {
                    depth -= 1;
                    if depth == 0 {
                        break j;
                    }
                }
                _ => {}
            }
        };
        // Callee is the identifier immediately before `(`.
        if open == 0 {
            return;
        }
        let callee = match self.peek(open - 1).map(|t| &t.token_type) {
            Some(TokenType::Identifier(s)) => s.clone(),
            _ => return,
        };
        if !is_predicate_bare(bare_name(&callee)) {
            return;
        }
        self.diag(
            format!("boolean comparison against `{word}`; use `{word}`"),
            lit.line,
            lit.column,
            lit.column + 1,
            &format!("replace with `{word}`"),
            word,
        );
    }

    pub fn run(&mut self) {
        let mut frames: Vec<FnFrame> = Vec::new();
        let mut depth = 0usize;
        let mut i = 0;
        while let Some(tok) = self.peek(i) {
            match &tok.token_type {
                TokenType::Function => {
                    if let Some(header) = self.parse_header(i) {
                        frames.push(FnFrame {
                            bare: bare_name(&header.name).to_string(),
                            ret_ann: header.ret_ann,
                            ann_span: header.ann_span,
                            depth_base: depth + 1,
                            saw_return: false,
                            all_bool: true,
                        });
                        depth += 1;
                        i = header.body_start;
                        continue;
                    }
                    i += 1;
                }
                TokenType::If
                | TokenType::While
                | TokenType::For
                | TokenType::When
                | TokenType::Try
                | TokenType::Repeat
                | TokenType::Struct
                | TokenType::Extern => {
                    depth += 1;
                    i += 1;
                }
                TokenType::FatArrow => {
                    // Multi-line closure bodies (`|x| =>` + newline) are
                    // closed by `end`; single-line arms are not blocks.
                    if matches!(
                        self.peek(i + 1).map(|t| &t.token_type),
                        Some(TokenType::Newline)
                    ) {
                        depth += 1;
                    }
                    i += 1;
                }
                TokenType::End => {
                    if depth == 0 {
                        // Unbalanced input: go silent rather than risk a
                        // wrong attribution.
                        break;
                    }
                    depth -= 1;
                    while frames.last().is_some_and(|f| depth < f.depth_base) {
                        let frame = frames.pop().unwrap();
                        self.finish_function(&frame);
                    }
                    i += 1;
                }
                TokenType::Return => {
                    if frames.is_empty() {
                        i += 1;
                        continue;
                    }
                    let (shape, next) = self.return_shape(i + 1);
                    if let Some(frame) = frames.last_mut() {
                        frame.saw_return = true;
                        if shape != Shape::Bool {
                            frame.all_bool = false;
                        } else if let Some(lit_idx) = self.bare_literal_after_return(i + 1) {
                            let eligible = is_predicate_bare(&frame.bare)
                                || is_bool_annotation(&frame.ret_ann);
                            if eligible {
                                let lit = &self.tokens[lit_idx];
                                let (digit, word) =
                                    if matches!(lit.token_type, TokenType::Number(1)) {
                                        ("1", "true")
                                    } else {
                                        ("0", "false")
                                    };
                                self.diag(
                                    format!("boolean return value `{digit}`; use `{word}`"),
                                    lit.line,
                                    lit.column,
                                    lit.column + 1,
                                    &format!("replace with `{word}`"),
                                    word,
                                );
                            }
                        }
                    }
                    // Skip past the value so its tokens are not
                    // re-scanned as comparisons.
                    i = next;
                    continue;
                }
                TokenType::Equal | TokenType::NotEqual => {
                    self.check_call_comparison(i);
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }
    }

    fn finish_function(&mut self, frame: &FnFrame) {
        if !frame.saw_return || !frame.all_bool {
            return;
        }
        if frame.ret_ann.as_deref() != Some("int") {
            return;
        }
        if !is_predicate_bare(&frame.bare) {
            return;
        }
        let (line, col) = match frame.ann_span {
            Some(s) => s,
            None => return,
        };
        self.diag(
            format!(
                "predicate function `{}` returns only boolean values; use `-> bool`",
                frame.bare
            ),
            line,
            col,
            col + 3,
            "replace with `-> bool`",
            "bool",
        );
    }
}

/// Checks for `1`/`0` used as booleans (`boolean-literals`, informational).
pub fn check_boolean_literals(tokens: &[Token], file_path: &Path) -> Vec<LintDiagnostic> {
    let mut scanner = Scanner {
        tokens,
        diags: Vec::new(),
        file_path,
    };
    scanner.run();
    scanner.diags
}
