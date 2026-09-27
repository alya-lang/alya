use super::protocol::{
    CompletionItem, Diagnostic, DocumentSymbol, FoldingRange, InlayHint, InlayHintKind, Location,
    ParameterInformation, Position, Range, RawSemanticToken, SemanticTokens, SignatureHelp,
    SignatureInformation, TextEdit, WorkspaceEdit,
};
use crate::ast::Stmt;
use crate::lexer::{Lexer, Token, TokenType};
use crate::parser::Parser;
use std::collections::{HashMap, HashSet};

pub fn check_document(source: &str, file_path: Option<&std::path::Path>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(err) => {
            diagnostics.push(parse_error_to_diagnostic(&err, source));
            return diagnostics;
        }
    };

    let default_path = std::path::Path::new("document.alya");
    let target_path = file_path.unwrap_or(default_path);

    let mut parser = Parser::new(tokens.clone());
    match parser.parse() {
        Ok(program) => {
            // 1. Static Gradual Type Checking Pass
            let mut resolved_ast = program.clone();
            crate::parser::enums::resolve_enums(&mut resolved_ast);
            let _ = crate::parser::constants::resolve_and_validate_constants(&mut resolved_ast);
            crate::parser::generics::resolve_generics(&mut resolved_ast);
            if let Err(type_err) =
                crate::codegen::analysis::type_checker::validate_types(&resolved_ast)
            {
                diagnostics.push(type_error_to_diagnostic(&type_err, source));
            }

            // 2. Linter Analysis Rules (same filters as the `alya lint` CLI:
            // inline suppressions plus project config from `.alyalint` /
            // `alya.toml [lint]`, discovered fresh per request so config
            // edits apply without restarting the server).
            let lint_diags = crate::tools::lint::run_all_rules(&program, &tokens, target_path);
            let suppression = crate::tools::lint::SuppressionFilter::from_source(source);
            let lint_diags = suppression.filter_diagnostics(lint_diags);
            let lint_config = crate::tools::lint::LintConfig::discover(target_path);
            let lint_diags = lint_config.apply_to_diagnostics(lint_diags);
            for d in lint_diags {
                let start_line = if d.line > 0 { (d.line - 1) as u32 } else { 0 };
                let start_col = if d.col > 0 { (d.col - 1) as u32 } else { 0 };
                let end_line = if d.end_line > 0 {
                    (d.end_line - 1) as u32
                } else {
                    start_line
                };
                let end_col = if d.end_col > 0 {
                    (d.end_col - 1) as u32
                } else {
                    start_col + 1
                };
                let range = Range::new(
                    Position::new(start_line, start_col),
                    Position::new(end_line, end_col),
                );
                let message = if let Some(help) = &d.help {
                    format!("{}\nhelp: {}", d.message, help)
                } else {
                    d.message.clone()
                };
                let severity = match d.severity {
                    crate::tools::lint::LintSeverity::Warning => 2,
                    crate::tools::lint::LintSeverity::Info => 3,
                    crate::tools::lint::LintSeverity::Error => 1,
                };
                diagnostics.push(Diagnostic {
                    range,
                    severity,
                    code: Some(d.rule),
                    message,
                    source: "alya-lint".to_string(),
                });
            }
        }
        Err(err) => {
            diagnostics.push(parse_error_to_diagnostic(&err, source));
        }
    }

    diagnostics
}

fn parse_error_to_diagnostic(err: &str, source: &str) -> Diagnostic {
    let mut line = 1;
    let mut col = 1;

    // Alya errors typically contain: "at line X, column Y" or similar
    if let Some(idx) = err.find("line ") {
        let remainder = &err[idx + 5..];
        let num_str: String = remainder
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(l) = num_str.parse::<u32>() {
            line = l;
        }
    }
    if let Some(idx) = err.find("column ") {
        let remainder = &err[idx + 7..];
        let num_str: String = remainder
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(c) = num_str.parse::<u32>() {
            col = c;
        }
    }

    let l_idx = if line > 0 { line - 1 } else { 0 };
    let c_idx = if col > 0 { col - 1 } else { 0 };

    let end_col = if let Some(line_str) = source.lines().nth(l_idx as usize) {
        (c_idx + 6).min(line_str.len() as u32)
    } else {
        c_idx + 6
    };

    Diagnostic::error(Range::single_line(l_idx, c_idx, end_col), err.to_string())
}

fn type_error_to_diagnostic(err: &str, source: &str) -> Diagnostic {
    let mut target_word = None;
    if let Some(idx) = err.find("in 'let ") {
        let remainder = &err[idx + 8..];
        if let Some(end) = remainder.find('\'') {
            target_word = Some(remainder[..end].trim());
        }
    } else if let Some(idx) = err.find("of 'let ") {
        let remainder = &err[idx + 8..];
        if let Some(end) = remainder.find('\'') {
            target_word = Some(remainder[..end].trim());
        }
    } else if let Some(idx) = err.find("to variable '") {
        let remainder = &err[idx + 13..];
        if let Some(end) = remainder.find('\'') {
            target_word = Some(remainder[..end].trim());
        }
    } else if let Some(idx) = err.find("function '") {
        let remainder = &err[idx + 10..];
        if let Some(end) = remainder.find('\'') {
            target_word = Some(remainder[..end].trim());
        }
    } else if let Some(idx) = err.find("field '") {
        let remainder = &err[idx + 7..];
        if let Some(end) = remainder.find('\'') {
            let field_part = remainder[..end].trim();
            if let Some((_, field)) = field_part.split_once('.') {
                target_word = Some(field);
            } else {
                target_word = Some(field_part);
            }
        }
    } else if let Some(idx) = err.find("struct '") {
        let remainder = &err[idx + 8..];
        if let Some(end) = remainder.find('\'') {
            target_word = Some(remainder[..end].trim());
        }
    }

    let mut line_idx = 0;
    let mut col_idx = 0;
    let mut end_col = 10;

    if let Some(word) = target_word {
        let let_pattern = format!("let {}", word);
        for (l, line_str) in source.lines().enumerate() {
            if let Some(c) = line_str.find(&let_pattern) {
                line_idx = l as u32;
                col_idx = c as u32;
                end_col = (c + let_pattern.len()) as u32;
                break;
            } else if let Some(c) = line_str.find(word) {
                line_idx = l as u32;
                col_idx = c as u32;
                end_col = (c + word.len()) as u32;
                break;
            }
        }
    } else if err.contains("Return") || err.contains("return") {
        for (l, line_str) in source.lines().enumerate() {
            if let Some(c) = line_str.find("return") {
                line_idx = l as u32;
                col_idx = c as u32;
                end_col = (c + 6) as u32;
                break;
            }
        }
    }

    let range = Range::new(
        Position::new(line_idx, col_idx),
        Position::new(line_idx, end_col),
    );

    Diagnostic {
        range,
        severity: 1, // Error
        code: Some("type-error".to_string()),
        message: err.to_string(),
        source: "alya-typecheck".to_string(),
    }
}

pub fn get_completions(source: &str, _pos: &Position) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // 1. Language Keywords (Kind 14)
    let keywords = [
        ("function", "Declares a function or method\n```alya\nfunction foo() -> int\n    return 42\nend\n```"),
        ("let", "Declares a mutable or immutable variable\n```alya\nlet x = 10\n```"),
        ("const", "Declares a compile-time constant\n```alya\nconst PI = 3.14159\n```"),
        ("struct", "Defines a structured data type\n```alya\nstruct Point\n    x: int\n    y: int\nend\n```"),
        ("interface", "Defines a structural duck-typed interface\n```alya\ninterface Drawable\n    function draw(self)\nend\n```"),
        ("enum", "Defines an enumeration with optional values\n```alya\nenum Color\n    Red\n    Green\n    Blue\nend\n```"),
        ("if", "Conditional branch\n```alya\nif condition\n    say \"yes\"\nend\n```"),
        ("else", "Alternative conditional branch"),
        ("while", "Loop while condition holds"),
        ("repeat", "Infinite or repeated loop"),
        ("for", "Range or iterator loop\n```alya\nfor i in 1..10\n    say i\nend\n```"),
        ("when", "Pattern matching switch construct"),
        ("is", "Dynamic or static type query operator"),
        ("say", "Prints expression to standard output"),
        ("return", "Returns from current function"),
        ("break", "Breaks out of innermost loop"),
        ("continue", "Continues to next loop iteration"),
        ("import", "Imports standard or external module"),
        ("spawn", "Spawns a concurrent green fiber"),
        ("select", "Multiplexes concurrent channels"),
        ("defer", "Defers statement execution to function exit"),
        ("weak", "Declares a cycle-breaking ARC weak reference"),
        ("pub", "Exports symbol for external modules"),
    ];

    for (kw, doc) in keywords {
        items.push(CompletionItem::new(kw, 14, Some("keyword"), Some(doc)));
    }

    // 2. Standard Library Modules (Kind 9)
    let std_modules = [
        "std/math",
        "std/simd",
        "std/mem",
        "std/fs",
        "std/os",
        "std/sync",
        "std/time",
        "std/net",
        "std/io",
        "std/json",
        "std/crypto",
        "std/env",
        "std/process",
        "std/path",
        "std/color",
        "std/thread",
    ];
    for m in std_modules {
        items.push(CompletionItem::new(
            m,
            9,
            Some("module"),
            Some("Alya Standard Library Module"),
        ));
    }

    // 3. Primitive & SIMD Vector Types (Kind 7)
    let types = [
        ("int", "64-bit signed integer"),
        ("float", "64-bit IEEE 754 floating-point number"),
        ("string", "UTF-8 encoded dynamic string"),
        ("bool", "Boolean value (`true` or `false`)"),
        ("ptr", "Raw native pointer"),
        ("any", "Dynamic type container with gradual typing"),
        ("void", "Absence of a return value"),
        (
            "f64x4",
            "256-bit SIMD vector of 4x 64-bit floats (`std/simd`)",
        ),
        (
            "f32x8",
            "256-bit SIMD vector of 8x 32-bit single-precision floats (`std/simd`)",
        ),
        (
            "i32x8",
            "256-bit SIMD vector of 8x 32-bit integers (`std/simd`)",
        ),
        (
            "i64x4",
            "256-bit SIMD vector of 4x 64-bit integers (`std/simd`)",
        ),
    ];
    for (t, doc) in types {
        items.push(CompletionItem::new(t, 7, Some("type"), Some(doc)));
    }

    // 3. User Definitions from AST (Functions: 3, Structs: 22, Interfaces: 8)
    let mut seen = HashSet::new();
    let mut lexer = Lexer::new(source);
    if let Ok(tokens) = lexer.tokenize() {
        let mut parser = Parser::new(tokens);
        if let Ok(ast) = parser.parse() {
            for stmt in &ast.statements {
                match stmt.inner_stmt() {
                    Stmt::Function {
                        name,
                        params,
                        return_type,
                        ..
                    } if seen.insert(name.clone()) => {
                        let ret = return_type.as_deref().unwrap_or("void");
                        let sig = format!("function {}({}) -> {}", name, params.join(", "), ret);
                        items.push(CompletionItem::new(name, 3, Some(&sig), Some(&sig)));
                    }
                    Stmt::StructDef { name, fields, .. } if seen.insert(name.clone()) => {
                        let detail = format!("struct {} ({})", name, fields.join(", "));
                        items.push(CompletionItem::new(name, 22, Some(&detail), Some(&detail)));
                    }
                    Stmt::InterfaceDef { name, methods, .. } if seen.insert(name.clone()) => {
                        let detail = format!("interface {} ({} methods)", name, methods.len());
                        items.push(CompletionItem::new(name, 8, Some(&detail), Some(&detail)));
                    }
                    Stmt::Let { name, type_ann, .. } if seen.insert(name.clone()) => {
                        let t = type_ann.as_deref().unwrap_or("auto");
                        let detail = format!("let {}: {}", name, t);
                        items.push(CompletionItem::new(name, 6, Some(&detail), None));
                    }
                    _ => {}
                }
            }
        }
    }

    items
}

pub fn get_hover(source: &str, pos: &Position) -> Option<String> {
    let word = get_word_at_pos(source, pos)?;

    // Check keywords
    let kw_doc = match word.as_str() {
        "function" => Some("**function**: Declares a named function or struct method.\n\n`function name(params) -> ReturnType`"),
        "struct" => Some("**struct**: Defines a heap-allocated ARC record type.\n\n`struct Name ... end`"),
        "interface" => Some("**interface**: Defines a duck-typing interface protocol.\n\n`interface Name ... end`"),
        "spawn" => Some("**spawn**: Spawns a concurrent green fiber executed over native thread workers.\n\n`spawn worker_func(arg)`"),
        "select" => Some("**select**: Channel multiplexing statement for CSP concurrency."),
        "weak" => Some("**weak**: Declares a cycle-breaking ARC weak reference."),
        "say" => Some("**say**: Built-in statement that writes formatted text to standard output."),
        "defer" => Some("**defer**: Schedules statement execution to the moment the enclosing function returns."),
        "when" => Some("**when**: S-expression pattern matching switch statement."),
        "is" => Some("**is**: Type query and interface satisfaction operator.\n\n`val is Interface`"),
        "Channel" => Some("**struct Channel[T]**: Thread-safe CSP communication channel with bounded & rendezvous semantics."),
        "Mutex" => Some("**struct Mutex**: Mutual exclusion primitive from `std/sync`."),
        "WaitGroup" => Some("**struct WaitGroup**: Thread synchronization counter from `std/sync`."),
        "f64x4" => Some("**struct f64x4**: 256-bit hardware SIMD vector containing 4x 64-bit IEEE 754 floating-point numbers (`std/simd`).\n\nSupports single-cycle parallel arithmetic (`+`, `-`, `*`, `/`), fused multiply-add (`.fma()`), and horizontal reductions (`.sum_horizontal()`, `.min()`, `.max()`)."),
        "f32x8" => Some("**struct f32x8**: 256-bit hardware SIMD vector containing 8x 32-bit single-precision floating-point numbers (`std/simd`).\n\nSupports parallel arithmetic (`+`, `-`, `*`, `/`), lane access (`.get()`, `.set()`), memory transfer (`.load()`, `.store()`), and horizontal reductions (`.sum_horizontal()`, `.min()`, `.max()`)."),
        "i32x8" => Some("**struct i32x8**: 256-bit hardware SIMD vector containing 8x 32-bit signed integers (`std/simd`).\n\nSupports 8-lane parallel integer arithmetic (`+`, `-`, `*`) and horizontal reductions (`.sum_horizontal()`, `.min()`, `.max()`)."),
        "i64x4" => Some("**struct i64x4**: 256-bit hardware SIMD vector containing 4x 64-bit signed integers (`std/simd`).\n\nSupports 4-lane parallel 64-bit integer arithmetic (`+`, `-`) and horizontal reductions (`.sum_horizontal()`, `.min()`, `.max()`)."),
        "Tensor" => Some("**struct Tensor**: Multi-dimensional contiguous numeric tensor with SIMD acceleration support (`alya-lang/tensor`)."),
        _ => None,
    };
    if let Some(doc) = kw_doc {
        return Some(doc.to_string());
    }

    // Check user definitions
    let mut lexer = Lexer::new(source);
    if let Ok(tokens) = lexer.tokenize() {
        let mut parser = Parser::new(tokens);
        if let Ok(ast) = parser.parse() {
            for stmt in &ast.statements {
                match stmt.inner_stmt() {
                    Stmt::Function {
                        name,
                        params,
                        return_type,
                        ..
                    } if name == &word => {
                        let ret = return_type.as_deref().unwrap_or("void");
                        return Some(format!(
                            "```alya\nfunction {}({}) -> {}\n```",
                            name,
                            params.join(", "),
                            ret
                        ));
                    }
                    Stmt::StructDef { name, fields, .. } if name == &word => {
                        return Some(format!(
                            "```alya\nstruct {}\n    {}\nend\n```",
                            name,
                            fields.join("\n    ")
                        ));
                    }
                    Stmt::InterfaceDef { name, methods, .. } if name == &word => {
                        let m_names: Vec<String> = methods
                            .iter()
                            .map(|m| format!("function {}({})", m.name, m.params.join(", ")))
                            .collect();
                        return Some(format!(
                            "```alya\ninterface {}\n    {}\nend\n```",
                            name,
                            m_names.join("\n    ")
                        ));
                    }
                    Stmt::Let { name, type_ann, .. } if name == &word => {
                        let t = type_ann.as_deref().unwrap_or("inferred");
                        return Some(format!("```alya\nlet {}: {}\n```", name, t));
                    }
                    _ => {}
                }
            }
        }
    }

    None
}

/// One `import "..." [as alias]` / `from "..." import ...` statement found by
/// line scan. Positions are byte offsets into the line (import lines are
/// matched on ASCII structure, so byte == char here in practice).
pub struct FileImport {
    pub line: u32,
    pub path: String,
    /// Byte range of the path text inside its quotes.
    pub path_start: usize,
    pub path_end: usize,
    pub alias: Option<String>,
    /// Byte offset of the alias token on the line (when `as alias` present).
    pub alias_start: usize,
    /// True for `from "..." import ...` lines (bare symbols enter scope).
    pub is_from: bool,
    /// Symbols of a `from` import: name, alias, and their byte offsets.
    pub symbols: Vec<FileImportSymbol>,
}

/// One `name [as alias]` entry of a `from "..." import ...` statement.
pub struct FileImportSymbol {
    pub name: String,
    pub alias: Option<String>,
    pub name_start: usize,
    pub alias_start: usize,
}

/// Where a go-to-definition request should land.
pub enum DefinitionTarget {
    /// Same document, at the given position (existing symbol behavior).
    SameFile(Position),
    /// A different file on disk; `pos` is the symbol position when known,
    /// otherwise clients open the file at its start.
    ExternalFile {
        path: std::path::PathBuf,
        pos: Option<Position>,
    },
}

/// Scans `import` / `from ... import` statements without a full parse.
///
/// Only double- or single-quoted module paths are collected; `from`
/// statements with bare-identifier paths are skipped (rare, and the
/// compiler resolves them through the same machinery as quoted ones).
pub fn parse_file_imports(source: &str) -> Vec<FileImport> {
    let mut out = Vec::new();
    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        let is_import = trimmed.starts_with("import ") || trimmed.starts_with("import\t");
        let is_from = trimmed.starts_with("from ") || trimmed.starts_with("from\t");
        if !is_import && !is_from {
            continue;
        }
        let bytes = line.as_bytes();
        // First quoted span on the line is the module path.
        let mut span: Option<(usize, usize, String)> = None;
        let mut i = 0;
        while i < bytes.len() {
            let q = bytes[i];
            if q == b'"' || q == b'\'' {
                if let Some(end) = line[i + 1..].find(q as char) {
                    span = Some((i + 1, i + 1 + end, line[i + 1..i + 1 + end].to_string()));
                    break;
                }
            }
            i += 1;
        }
        let (path_start, path_end, path) = match span {
            Some(s) => s,
            None => continue,
        };
        // Optional `as alias` after the closing quote (plain `import` form).
        let mut alias: Option<String> = None;
        let mut alias_start = 0usize;
        if is_import {
            let rest = &line[path_end + 1..];
            if let Some(as_pos) = find_as_keyword(rest) {
                let after = rest[as_pos + 2..].trim_start();
                let name: String = after
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    let leading_ws =
                        rest[as_pos + 2..].len() - rest[as_pos + 2..].trim_start().len();
                    alias_start = path_end + 1 + as_pos + 2 + leading_ws;
                    alias = Some(name);
                }
            }
        }
        out.push(FileImport {
            line: line_idx as u32,
            path,
            path_start,
            path_end,
            alias,
            alias_start,
            is_from,
            symbols: if is_from {
                parse_from_symbols(line, path_end + 1)
            } else {
                Vec::new()
            },
        });
    }
    out
}

/// Parses the `import a, b as c, ...` symbol list of a `from` line,
/// starting the scan at byte offset `from`. Records byte offsets of each
/// name and alias token. `*` entries are skipped (no word to click).
fn parse_from_symbols(line: &str, from: usize) -> Vec<FileImportSymbol> {
    let bytes = line.as_bytes();
    // The symbol list starts after the `import` keyword following the path.
    let mut i = from;
    // Skip to end of the `import` keyword: first identifier-like word that
    // is exactly `import`.
    let mut found = false;
    while i < bytes.len() {
        while i < bytes.len() && !(bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        if start < i && &line[start..i] == "import" {
            found = true;
            break;
        }
        if start == i {
            i += 1;
        }
    }
    if !found {
        return Vec::new();
    }
    let mut out = Vec::new();
    loop {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'*' {
            i += 1;
            continue;
        }
        if !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
            break;
        }
        let name_start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        let name = line[name_start..i].to_string();
        let mut alias = None;
        let mut alias_start = name_start;
        let save = i;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if line[i..].starts_with("as")
            && (i + 2 >= bytes.len()
                || !(bytes[i + 2].is_ascii_alphanumeric() || bytes[i + 2] == b'_'))
        {
            i += 2;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
                alias_start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                alias = Some(line[alias_start..i].to_string());
            } else {
                i = save;
            }
        } else {
            i = save;
        }
        out.push(FileImportSymbol {
            name,
            alias,
            name_start,
            alias_start,
        });
    }
    out
}

/// Finds a standalone `as` keyword in `rest` (not part of an identifier).
fn find_as_keyword(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i + 2 <= bytes.len() {
        if &bytes[i..i + 2] == b"as"
            && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'))
            && (i + 2 >= bytes.len()
                || !(bytes[i + 2].is_ascii_alphanumeric() || bytes[i + 2] == b'_'))
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Byte range of the identifier-like word under `col` on `line`.
fn word_range_at(line: &str, col: usize) -> Option<(usize, usize)> {
    let bytes = line.as_bytes();
    if bytes.is_empty() || col >= bytes.len() {
        return None;
    }
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    if !is_word(bytes[col.min(bytes.len() - 1)]) {
        return None;
    }
    let mut start = col.min(bytes.len() - 1);
    while start > 0 && is_word(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = start;
    while end < bytes.len() && is_word(bytes[end]) {
        end += 1;
    }
    Some((start, end))
}

/// Lexically normalizes `.` / `..` segments without touching the filesystem
/// (unlike `fs::canonicalize`, this never produces `\\?\`-style paths, so
/// the result stays usable for `file://` URIs).
fn lexical_normalize(path: std::path::PathBuf) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(out.components().next_back(), Some(Component::Normal(_))) {
                    out.pop();
                } else {
                    out.push(comp.as_os_str());
                }
            }
            _ => out.push(comp.as_os_str()),
        }
    }
    out
}
/// Resolves an import path string to a file on disk, mirroring the
/// compiler's resolution order (`parser::resolve_stmt_imports_ext_with_rewrites`):
/// relative path (against the importing file's directory), `std/...` against
/// nearby `stdlib/` roots, otherwise the installed package entry. Returns
/// `None` for embedded-stdlib modules, which have no on-disk file.
pub fn resolve_import_to_file(
    import_path: &str,
    file_dir: Option<&std::path::Path>,
) -> Option<std::path::PathBuf> {
    let normalized = import_path.replace('\\', "/");
    let path = std::path::Path::new(&normalized);
    // NOTE: every branch below must yield an absolute path (or None):
    // relative results would produce bogus `file://` URIs downstream.
    let target = if path.is_absolute() {
        path.to_path_buf()
    } else if let Some(dir) = file_dir {
        dir.join(path)
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(path)
    } else {
        path.to_path_buf()
    };
    if target.exists() {
        return Some(lexical_normalize(target));
    }
    if target.with_extension("alya").exists() {
        return Some(lexical_normalize(target.with_extension("alya")));
    }
    if normalized.starts_with("std/") || normalized.starts_with("std::") {
        let clean = normalized
            .strip_prefix("std/")
            .or_else(|| normalized.strip_prefix("std::"))
            .unwrap_or(&normalized);
        let clean = clean.strip_suffix(".alya").unwrap_or(clean);
        let canonical = crate::parser::canonical_stdlib_module(clean);
        let mut roots: Vec<std::path::PathBuf> = Vec::new();
        if let Some(dir) = file_dir {
            roots.push(dir.join("stdlib").join(canonical));
        }
        // Process-cwd root, joined absolutely: a bare `stdlib/...` result
        // would otherwise escape as a relative `file://` URI.
        if let Ok(cwd) = std::env::current_dir() {
            roots.push(cwd.join("stdlib").join(canonical));
        }
        for root in roots {
            if root.exists() {
                return Some(root);
            }
            if root.with_extension("alya").exists() {
                return Some(root.with_extension("alya"));
            }
        }
        // Falls back to embedded stdlib inside the compiler: no file.
        return None;
    }
    if let Some(dir) = file_dir {
        if let Ok(Some(pkg_path)) = crate::tools::pkg::resolve_package_import(&normalized, dir) {
            return Some(lexical_normalize(pkg_path));
        }
    }
    None
}

/// Declaration kind of a symbol found by the text scan.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Interface,
    Enum,
    Let,
}

/// Same-file text scan for a declared symbol name: functions (including
/// `Type.method` methods), structs, interfaces, enums, `let`/`const`.
pub fn find_symbol_in_source(source: &str, word: &str) -> Option<(Position, SymbolKind)> {
    for (line_idx, line) in source.lines().enumerate() {
        let mut trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("pub ") {
            trimmed = rest.trim_start();
        }
        if let Some(rest) = trimmed.strip_prefix("function ") {
            let rest = rest.trim_start();
            let fn_name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
                .collect();
            if fn_name == word
                || fn_name.ends_with(&format!(".{}", word))
                || fn_name.ends_with(&format!("__{}", word))
            {
                let char_idx = line.find(word).unwrap_or(0) as u32;
                return Some((
                    Position::new(line_idx as u32, char_idx),
                    SymbolKind::Function,
                ));
            }
        } else if let Some(rest) = trimmed.strip_prefix("struct ") {
            let rest = rest.trim_start();
            let st_name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if st_name == word {
                let char_idx = line.find(word).unwrap_or(0) as u32;
                return Some((Position::new(line_idx as u32, char_idx), SymbolKind::Struct));
            }
        } else if let Some(rest) = trimmed.strip_prefix("interface ") {
            let rest = rest.trim_start();
            let if_name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if if_name == word {
                let char_idx = line.find(word).unwrap_or(0) as u32;
                return Some((
                    Position::new(line_idx as u32, char_idx),
                    SymbolKind::Interface,
                ));
            }
        } else if let Some(rest) = trimmed.strip_prefix("enum ") {
            let rest = rest.trim_start();
            let en_name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if en_name == word {
                let char_idx = line.find(word).unwrap_or(0) as u32;
                return Some((Position::new(line_idx as u32, char_idx), SymbolKind::Enum));
            }
        } else if let Some(rest) = trimmed
            .strip_prefix("let ")
            .or_else(|| trimmed.strip_prefix("const "))
        {
            let rest = rest.trim_start();
            let var_name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if var_name == word {
                let char_idx = line.find(word).unwrap_or(0) as u32;
                return Some((Position::new(line_idx as u32, char_idx), SymbolKind::Let));
            }
        }
    }

    None
}

/// Locates `variant` inside the `enum enum_name` block (same-file scan).
pub fn find_enum_variant_in_source(
    source: &str,
    enum_name: &str,
    variant: &str,
) -> Option<Position> {
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let mut trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("pub ") {
            trimmed = rest.trim_start();
        }
        if let Some(rest) = trimmed.strip_prefix("enum ") {
            let name: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if name == enum_name {
                i += 1;
                while i < lines.len() {
                    let vline = lines[i];
                    let vt = vline.trim_start();
                    if vt == "end" || vt.starts_with("end ") || vt.starts_with("end\t") {
                        break;
                    }
                    // Ignore trailing comments; the head token must be the
                    // full identifier, so `Active` never matches `ActiveX`.
                    let code = vt.split('#').next().unwrap_or("");
                    let head: String = code
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !head.is_empty() && head == variant {
                        let char_idx = vline.find(variant).unwrap_or(0) as u32;
                        return Some(Position::new(i as u32, char_idx));
                    }
                    i += 1;
                }
                return None;
            }
        }
        i += 1;
    }
    None
}

/// Parses the `a::b.c` qualifier chain around the cursor (whitespace around
/// separators tolerated). Returns the segments and the index of the segment
/// under the cursor. All slice points are ASCII bytes, so this is
/// panic-free on non-ASCII lines.
pub fn qualifier_chain_at(line: &str, col: usize) -> Option<(Vec<String>, usize)> {
    let (start, end) = word_range_at(line, col)?;
    let bytes = line.as_bytes();
    let is_word_byte = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut segments = vec![line[start..end].to_string()];
    let mut cursor_idx = 0usize;
    // Expand left over `[ws] (:: | .) [ws] ident`.
    let mut pos = start;
    loop {
        let mut p = pos;
        while p > 0 && bytes[p - 1].is_ascii_whitespace() {
            p -= 1;
        }
        let sep_len = if p >= 2 && bytes[p - 2] == b':' && bytes[p - 1] == b':' {
            2
        } else if p >= 1 && bytes[p - 1] == b'.' {
            1
        } else {
            break;
        };
        let mut q = p - sep_len;
        while q > 0 && bytes[q - 1].is_ascii_whitespace() {
            q -= 1;
        }
        let mut s = q;
        while s > 0 && is_word_byte(bytes[s - 1]) {
            s -= 1;
        }
        if s == q {
            break;
        }
        segments.insert(0, line[s..q].to_string());
        cursor_idx += 1;
        pos = s;
    }
    // Expand right over ident `[ws] (:: | .) [ws]`.
    let mut pos = end;
    loop {
        let mut p = pos;
        while p < bytes.len() && bytes[p].is_ascii_whitespace() {
            p += 1;
        }
        let sep_len = if p + 1 < bytes.len() && bytes[p] == b':' && bytes[p + 1] == b':' {
            2
        } else if p < bytes.len() && bytes[p] == b'.' {
            1
        } else {
            break;
        };
        let mut q = p + sep_len;
        while q < bytes.len() && bytes[q].is_ascii_whitespace() {
            q += 1;
        }
        let mut e = q;
        while e < bytes.len() && is_word_byte(bytes[e]) {
            e += 1;
        }
        if e == q {
            break;
        }
        segments.push(line[q..e].to_string());
        pos = e;
    }
    Some((segments, cursor_idx))
}

/// Searches `name` in the given file and, when absent, in the files its
/// scope-merging (unaliased + `from`) imports point to. Returns the file
/// where the symbol was found, its position, and its kind.
///
/// This mirrors how facades work: `pkg::X` often names a symbol declared in
/// a module the facade itself imports. Cycles terminate via `visited`, and
/// depth is capped (import chains in practice are a few levels).
fn find_first_seg(
    search_path: &std::path::Path,
    name: &str,
    depth: u32,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Option<(std::path::PathBuf, Position, SymbolKind)> {
    if depth > 4 || !visited.insert(search_path.to_path_buf()) {
        return None;
    }
    let src = std::fs::read_to_string(search_path).ok()?;
    if let Some((pos, kind)) = find_symbol_in_source(&src, name) {
        return Some((search_path.to_path_buf(), pos, kind));
    }
    let dir = search_path.parent()?;
    for imp in parse_file_imports(&src) {
        if imp.alias.is_some() {
            continue;
        }
        let next = match resolve_import_to_file(&imp.path, Some(dir)) {
            Some(p) => p,
            None => continue,
        };
        if let Some(hit) = find_first_seg(&next, name, depth + 1, visited) {
            return Some(hit);
        }
    }
    None
}

/// Searches `name` through the scope-merging (unaliased + `from`) imports
/// of the current file, transitively. Aliased imports are skipped: they do
/// not merge bare names into scope.
fn find_in_unaliased_imports(
    imports: &[FileImport],
    name: &str,
    file_dir: Option<&std::path::Path>,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Option<(std::path::PathBuf, Position, SymbolKind)> {
    for imp in imports {
        if imp.alias.is_some() {
            continue;
        }
        let target = resolve_import_to_file(&imp.path, file_dir)?;
        if let Some(hit) = find_first_seg(&target, name, 0, visited) {
            return Some(hit);
        }
    }
    None
}

/// Locates `field` inside the `struct struct_name` block (same-file scan).
pub fn find_struct_field_in_source(
    source: &str,
    struct_name: &str,
    field: &str,
) -> Option<Position> {
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let mut trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("pub ") {
            trimmed = rest.trim_start();
        }
        if let Some(rest) = trimmed.strip_prefix("struct ") {
            let name: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if name == struct_name {
                i += 1;
                while i < lines.len() {
                    let fline = lines[i];
                    let ft = fline.trim_start();
                    if ft == "end" || ft.starts_with("end ") || ft.starts_with("end\t") {
                        break;
                    }
                    let code = ft.split('#').next().unwrap_or("");
                    let head: String = code
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !head.is_empty() && head == field {
                        let char_idx = fline.find(field).unwrap_or(0) as u32;
                        return Some(Position::new(i as u32, char_idx));
                    }
                    i += 1;
                }
                return None;
            }
        }
        i += 1;
    }
    None
}

/// Returns the identifier immediately before the last `{` in `code`
/// (e.g. `TlsServer` in `return TlsServer {`), if any.
fn opener_type_before_brace(code: &str) -> Option<String> {
    let brace = code.rfind('{')?;
    let before = &code[..brace];
    let ident: String = before
        .chars()
        .rev()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    let ident: String = ident.chars().rev().collect();
    if ident.is_empty() {
        None
    } else {
        Some(ident)
    }
}

/// Finds the struct literal enclosing the cursor (`Type { ... field ... }`),
/// returning the literal's type name. Handles same-line literals and
/// multi-line literals via brace-depth tracking upward (capped). Returns
/// `None` outside struct literals.
fn enclosing_struct_literal(source: &str, pos: &Position) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = pos.line as usize;
    if line_idx >= lines.len() {
        return None;
    }
    let col = (pos.character as usize).min(lines[line_idx].len());
    // Same-line rule: inside an unclosed `{` on this line before the cursor.
    let prefix = &lines[line_idx][..col];
    let opens = prefix.matches('{').count();
    let closes = prefix.matches('}').count();
    if opens > closes {
        let no_comment = prefix.split('#').next().unwrap_or(prefix);
        if let Some(t) = opener_type_before_brace(no_comment) {
            return Some(t);
        }
    }
    // Multi-line rule: walk upward tracking depth; the line that opens the
    // block containing the cursor names the type.
    let mut depth: i32 = 0;
    for l in lines[..line_idx].iter().rev().take(80) {
        let code = l.split('#').next().unwrap_or("");
        depth += code.matches('}').count() as i32 - code.matches('{').count() as i32;
        if depth < 0 {
            return opener_type_before_brace(code);
        }
    }
    None
}
///
/// Go-to-definition with import awareness.
///
/// - Cursor inside an import path string: the resolved target file.
/// - Cursor on an `alias` used as `alias::x` / `alias.x`: the import line.
/// - Cursor on `x` in `alias::x` / `alias.x`: the symbol in the target file
///   (enum variants resolve to their variant line).
/// - Cursor on `Variant` in `Enum::Variant`: the variant line (the enum may
///   itself live in an imported file).
/// - Cursor on a `from` symbol (name or `as` alias): the symbol in the
///   target file.
/// - Bare use of a `from`-imported symbol (f-string interpolations
///   included): the symbol on the `from` line.
/// - Cursor on a `field:` key inside a `Type { ... }` literal: the field
///   line of the struct (same file or imports).
/// - Bare words first use the same-file scan, then unaliased imports in
///   order (aliased imports do not merge bare names into scope, so they
///   are skipped there).
pub fn get_definition_target(
    source: &str,
    pos: &Position,
    file_dir: Option<&std::path::Path>,
) -> Option<DefinitionTarget> {
    let imports = parse_file_imports(source);
    let line = source.lines().nth(pos.line as usize)?;
    let col = pos.character as usize;

    // 1. Cursor on an import path literal -> target file.
    for imp in &imports {
        if imp.line == pos.line && col >= imp.path_start && col < imp.path_end {
            let target = resolve_import_to_file(&imp.path, file_dir)?;
            return Some(DefinitionTarget::ExternalFile {
                path: target,
                pos: None,
            });
        }
    }

    // 1b. Cursor on a `from` symbol (name or `as` alias) -> the symbol in
    // the target file. Unresolvable paths (embedded stdlib) yield no jump.
    for imp in &imports {
        if imp.line != pos.line {
            continue;
        }
        for sym in &imp.symbols {
            let on_name = col >= sym.name_start && col < sym.name_start + sym.name.len();
            let alias_len = sym.alias.as_ref().map_or(0, |a| a.len());
            let on_alias =
                alias_len > 0 && col >= sym.alias_start && col < sym.alias_start + alias_len;
            if on_name || on_alias {
                let target = resolve_import_to_file(&imp.path, file_dir)?;
                let target_src = std::fs::read_to_string(&target).ok()?;
                let (sym_pos, _) = find_symbol_in_source(&target_src, &sym.name)?;
                return Some(DefinitionTarget::ExternalFile {
                    path: target,
                    pos: Some(sym_pos),
                });
            }
        }
    }

    // Word under the cursor (via chain when qualified, plain otherwise).
    let (segments, cursor_idx) = qualifier_chain_at(line, col)
        .map(|(s, i)| (s, Some(i)))
        .unwrap_or_else(|| {
            (
                get_word_at_pos(source, pos).map_or(Vec::new(), |w| vec![w]),
                None,
            )
        });
    if segments.is_empty() {
        return None;
    }
    let word = segments[cursor_idx.unwrap_or(0)].clone();

    // 2. Qualified by an import alias.
    if segments.len() > 1 {
        if let Some(head) = segments.first() {
            if let Some(imp) = imports
                .iter()
                .find(|i| i.alias.as_deref() == Some(head.as_str()))
            {
                if cursor_idx == Some(0) {
                    // The alias token itself is not a usage; don't self-jump.
                    if imp.line == pos.line
                        && col >= imp.alias_start
                        && col < imp.alias_start + head.len()
                    {
                        return None;
                    }
                    return Some(DefinitionTarget::SameFile(Position::new(
                        imp.line,
                        imp.alias_start as u32,
                    )));
                }
                // Deep jump: resolve `alias::seg...` inside the target file,
                // stopping at the segment under the cursor (scope-merging
                // imports are followed, so facade re-exports resolve). A
                // qualified cursor is explicit cross-file intent: failure
                // yields no jump rather than a same-file guess.
                let target = resolve_import_to_file(&imp.path, file_dir)?;
                let end = cursor_idx.expect("alias qualifier") + 1;
                let segs = &segments[1..end];
                let mut visited = std::collections::HashSet::new();
                let (def_file, mut sym_pos, mut kind) =
                    find_first_seg(&target, &segs[0], 0, &mut visited)?;
                let def_src = std::fs::read_to_string(&def_file).ok()?;
                let mut prev_name = segs[0].clone();
                for next in &segs[1..] {
                    match kind {
                        SymbolKind::Enum => {
                            sym_pos = find_enum_variant_in_source(&def_src, &prev_name, next)?;
                            kind = SymbolKind::Let;
                        }
                        _ => {
                            // Best effort (e.g. `Type::method` falls back to
                            // the flat scan, which matches `Type.method`).
                            let (p, k) = find_symbol_in_source(&def_src, next)?;
                            sym_pos = p;
                            kind = k;
                        }
                    }
                    prev_name = next.clone();
                }
                return Some(DefinitionTarget::ExternalFile {
                    path: def_file,
                    pos: Some(sym_pos),
                });
            }
        }
    }

    // 3. Non-alias qualifier: `Enum::Variant` in the same file.
    if let Some(idx) = cursor_idx {
        if segments.len() > 1 && idx > 0 {
            let prev = &segments[idx - 1];
            if let Some((_, SymbolKind::Enum)) = find_symbol_in_source(source, prev) {
                if let Some(vpos) = find_enum_variant_in_source(source, prev, &word) {
                    return Some(DefinitionTarget::SameFile(vpos));
                }
            }
        }
    }

    // 4. Struct literal field keys (`Type { ... field: ... }`). Guarded by
    // the `field:` shape (not `::`), the enclosing literal, and the field
    // actually existing: anything else falls through to the scans below.
    if let Some((start, end)) = word_range_at(line, col) {
        let after_key = &line[end..];
        let mut chars = after_key.chars().skip_while(|c| c.is_whitespace());
        let is_key = matches!(chars.next(), Some(':')) && !matches!(chars.next(), Some(':'));
        if is_key {
            let field = line[start..end].to_string();
            if let Some(type_name) = enclosing_struct_literal(source, pos) {
                if let Some((_, SymbolKind::Struct)) = find_symbol_in_source(source, &type_name) {
                    if let Some(fpos) = find_struct_field_in_source(source, &type_name, &field) {
                        return Some(DefinitionTarget::SameFile(fpos));
                    }
                } else {
                    let mut visited = std::collections::HashSet::new();
                    if let Some((def_file, _, SymbolKind::Struct)) =
                        find_in_unaliased_imports(&imports, &type_name, file_dir, &mut visited)
                    {
                        let def_src = std::fs::read_to_string(&def_file).ok().unwrap_or_default();
                        if let Some(fpos) =
                            find_struct_field_in_source(&def_src, &type_name, &field)
                        {
                            return Some(DefinitionTarget::ExternalFile {
                                path: def_file,
                                pos: Some(fpos),
                            });
                        }
                    }
                }
            }
        }
    }

    // 5. Existing same-file behavior.
    if let Some(def_pos) = get_definition_pos(source, pos) {
        return Some(DefinitionTarget::SameFile(def_pos));
    }

    // 5b. Bare use of a `from`-imported symbol (or its `as` alias) -> the
    // symbol on the `from` line. Mirrors the alias -> import line jump;
    // from here the path/symbol hops reach the target file. Shadowing
    // locals already won in step 5. Effective name: alias if present.
    for imp in &imports {
        for sym in &imp.symbols {
            let effective = sym.alias.as_ref().unwrap_or(&sym.name);
            if effective == &word {
                let (tok_start, tok_len) = match &sym.alias {
                    Some(a) => (sym.alias_start, a.len()),
                    None => (sym.name_start, sym.name.len()),
                };
                // The symbol's own tokens on the from line are not usages.
                if imp.line == pos.line && col >= tok_start && col < tok_start + tok_len {
                    return None;
                }
                return Some(DefinitionTarget::SameFile(Position::new(
                    imp.line,
                    tok_start as u32,
                )));
            }
        }
    }

    // 6. Bare qualified use (`Prev.word`) where `Prev` is an enum reachable
    // through unaliased imports: jump to the variant line. Variants are not
    // top-level symbols, so the flat search below cannot find them.
    if let Some(idx) = cursor_idx {
        if segments.len() > 1 && idx > 0 {
            let prev = &segments[idx - 1];
            let mut visited = std::collections::HashSet::new();
            if let Some((def_file, _, SymbolKind::Enum)) =
                find_in_unaliased_imports(&imports, prev, file_dir, &mut visited)
            {
                let def_src = std::fs::read_to_string(&def_file).ok().unwrap_or_default();
                if let Some(vpos) = find_enum_variant_in_source(&def_src, prev, &word) {
                    return Some(DefinitionTarget::ExternalFile {
                        path: def_file,
                        pos: Some(vpos),
                    });
                }
            }
        }
    }

    // 7. Bare words fall back to unaliased imports in order (followed
    // transitively, like the qualified case). Aliased imports are skipped:
    // they do not merge bare names into scope.
    let mut visited = std::collections::HashSet::new();
    if let Some((def_file, p, _)) =
        find_in_unaliased_imports(&imports, &word, file_dir, &mut visited)
    {
        return Some(DefinitionTarget::ExternalFile {
            path: def_file,
            pos: Some(p),
        });
    }

    None
}

pub fn get_definition_pos(source: &str, pos: &Position) -> Option<Position> {
    let word = get_word_at_pos(source, pos)?;
    find_symbol_in_source(source, &word).map(|(p, _)| p)
}

pub fn get_word_at_pos(source: &str, pos: &Position) -> Option<String> {
    let line = source.lines().nth(pos.line as usize)?;
    let col = pos.character as usize;
    if col >= line.len() && col > 0 {
        return None;
    }

    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let target_idx = col.min(chars.len() - 1);
    if !is_ident_char(chars[target_idx]) {
        return None;
    }

    let mut start = target_idx;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }

    let mut end = target_idx;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }

    let word: String = chars[start..end].iter().collect();
    if word.is_empty() {
        None
    } else {
        Some(word)
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Discovers hierarchical document symbols for the outline and breadcrumbs views.
pub fn get_document_symbols(source: &str) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(_) => return symbols,
    };

    let mut parser = Parser::new(tokens.clone());
    let ast = match parser.parse() {
        Ok(a) => a,
        Err(_) => return symbols,
    };

    for stmt in &ast.statements {
        let is_pub = stmt.is_pub();
        let inner = stmt.inner_stmt();
        match inner {
            Stmt::Function {
                name,
                params,
                return_type,
                ..
            } => {
                let kind = if name.contains('.') || name.contains("::") {
                    6 // Method
                } else {
                    12 // Function
                };
                let ret = return_type.as_deref().unwrap_or("void");
                let detail = format!("({}) -> {}", params.join(", "), ret);
                if let Some((range, sel_range)) =
                    find_symbol_range(&tokens, is_pub, TokenType::Function, name, true)
                {
                    symbols.push(DocumentSymbol::new(
                        name,
                        Some(&detail),
                        kind,
                        range,
                        sel_range,
                    ));
                }
            }
            Stmt::StructDef { name, fields, .. } => {
                let detail = format!("struct ({} fields)", fields.len());
                if let Some((range, sel_range)) =
                    find_symbol_range(&tokens, is_pub, TokenType::Struct, name, true)
                {
                    let mut sym = DocumentSymbol::new(
                        name,
                        Some(&detail),
                        23, // Struct
                        range.clone(),
                        sel_range,
                    );
                    for field in fields {
                        if let Some((f_range, f_sel)) =
                            find_field_range(&tokens, range.clone(), field)
                        {
                            sym.children.push(DocumentSymbol::new(
                                field, None, 8, // Field
                                f_range, f_sel,
                            ));
                        }
                    }
                    symbols.push(sym);
                }
            }
            Stmt::EnumDef { name, variants } => {
                let detail = format!("enum ({} variants)", variants.len());
                if let Some((range, sel_range)) =
                    find_symbol_range(&tokens, is_pub, TokenType::Enum, name, true)
                {
                    let mut sym = DocumentSymbol::new(
                        name,
                        Some(&detail),
                        10, // Enum
                        range.clone(),
                        sel_range,
                    );
                    for (variant, _) in variants {
                        if let Some((v_range, v_sel)) =
                            find_field_range(&tokens, range.clone(), variant)
                        {
                            sym.children.push(DocumentSymbol::new(
                                variant, None, 22, // EnumMember
                                v_range, v_sel,
                            ));
                        }
                    }
                    symbols.push(sym);
                }
            }
            Stmt::InterfaceDef { name, methods, .. } => {
                let detail = format!("interface ({} methods)", methods.len());
                if let Some((range, sel_range)) =
                    find_symbol_range(&tokens, is_pub, TokenType::Interface, name, true)
                {
                    let mut sym = DocumentSymbol::new(
                        name,
                        Some(&detail),
                        11, // Interface
                        range.clone(),
                        sel_range,
                    );
                    for m in methods {
                        let m_sig = format!(
                            "({}) -> {}",
                            m.params.join(", "),
                            m.return_type.as_deref().unwrap_or("void")
                        );
                        if let Some((m_range, m_sel)) =
                            find_field_range(&tokens, range.clone(), &m.name)
                        {
                            sym.children.push(DocumentSymbol::new(
                                &m.name,
                                Some(&m_sig),
                                6, // Method
                                m_range,
                                m_sel,
                            ));
                        }
                    }
                    symbols.push(sym);
                }
            }
            Stmt::Const { name, .. } => {
                if let Some((range, sel_range)) =
                    find_symbol_range(&tokens, is_pub, TokenType::Const, name, false)
                {
                    symbols.push(DocumentSymbol::new(
                        name,
                        Some("const"),
                        14, // Constant
                        range,
                        sel_range,
                    ));
                }
            }
            _ => {}
        }
    }

    symbols
}

fn find_symbol_range(
    tokens: &[Token],
    _is_pub: bool,
    kw_type: TokenType,
    name: &str,
    has_end: bool,
) -> Option<(Range, Range)> {
    let simple_name = name.split('.').next_back().unwrap_or(name);
    let mut i = 0;
    while i < tokens.len() {
        let pub_tok = if matches!(tokens[i].token_type, TokenType::Pub) {
            let p = Some(&tokens[i]);
            i += 1;
            p
        } else {
            None
        };

        if i < tokens.len() && tokens[i].token_type == kw_type {
            let kw_tok = &tokens[i];
            let start_tok = pub_tok.unwrap_or(kw_tok);
            let start_line = start_tok.line.saturating_sub(1) as u32;
            let start_col = start_tok.column.saturating_sub(1) as u32;

            let mut name_found = false;
            let mut sel_start = Position::new(start_line, start_col);
            let mut sel_end = Position::new(start_line, start_col);

            let mut scan = i + 1;
            while scan < tokens.len() && scan <= i + 6 {
                if let TokenType::Identifier(ref id) = tokens[scan].token_type {
                    let first_part = name.split('.').next().unwrap_or(name);
                    if id == name || id == simple_name || id == first_part {
                        let l = tokens[scan].line.saturating_sub(1) as u32;
                        let c = tokens[scan].column.saturating_sub(1) as u32;
                        sel_start = Position::new(l, c);
                        sel_end = Position::new(l, c + name.len() as u32);
                        name_found = true;
                        break;
                    }
                }
                scan += 1;
            }

            if name_found {
                let end_pos = if has_end {
                    let mut depth = 1;
                    let mut end_p = sel_end.clone();
                    let mut forward = scan + 1;
                    while forward < tokens.len() {
                        match &tokens[forward].token_type {
                            TokenType::Function
                            | TokenType::Struct
                            | TokenType::Enum
                            | TokenType::Interface
                            | TokenType::If
                            | TokenType::While
                            | TokenType::For
                            | TokenType::Repeat
                            | TokenType::When
                            | TokenType::Try
                            | TokenType::Test
                            | TokenType::Bench => {
                                depth += 1;
                            }
                            TokenType::End => {
                                depth -= 1;
                                if depth == 0 {
                                    let el = tokens[forward].line.saturating_sub(1) as u32;
                                    let ec = tokens[forward].column.saturating_sub(1) as u32 + 3;
                                    end_p = Position::new(el, ec);
                                    break;
                                }
                            }
                            _ => {}
                        }
                        forward += 1;
                    }
                    end_p
                } else {
                    let mut forward = scan + 1;
                    let mut el = sel_end.line;
                    let mut ec = sel_end.character;
                    while forward < tokens.len() {
                        if matches!(tokens[forward].token_type, TokenType::Newline) {
                            el = tokens[forward].line.saturating_sub(1) as u32;
                            ec = tokens[forward].column.saturating_sub(1) as u32;
                            break;
                        }
                        forward += 1;
                    }
                    Position::new(el, ec)
                };

                let full_range = Range::new(Position::new(start_line, start_col), end_pos);
                let sel_range = Range::new(sel_start, sel_end);
                return Some((full_range, sel_range));
            }
        }
        i += 1;
    }
    None
}

fn find_field_range(
    tokens: &[Token],
    parent_range: Range,
    field_name: &str,
) -> Option<(Range, Range)> {
    for tok in tokens {
        let line = tok.line.saturating_sub(1) as u32;
        if line >= parent_range.start.line && line <= parent_range.end.line {
            if let TokenType::Identifier(ref id) = tok.token_type {
                if id == field_name {
                    let col = tok.column.saturating_sub(1) as u32;
                    let r = Range::new(
                        Position::new(line, col),
                        Position::new(line, col + id.len() as u32),
                    );
                    return Some((r.clone(), r));
                }
            }
        }
    }
    None
}

/// Discovers AST-driven and comment/import folding ranges for editors.
pub fn get_folding_ranges(source: &str) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();

    let mut lexer = Lexer::new(source);
    if let Ok(tokens) = lexer.tokenize() {
        let mut block_stack = Vec::new();
        for tok in &tokens {
            match &tok.token_type {
                TokenType::Function
                | TokenType::Struct
                | TokenType::Enum
                | TokenType::Interface
                | TokenType::If
                | TokenType::While
                | TokenType::For
                | TokenType::Repeat
                | TokenType::When
                | TokenType::Try
                | TokenType::Test
                | TokenType::Bench => {
                    let l = tok.line.saturating_sub(1) as u32;
                    block_stack.push(l);
                }
                TokenType::End => {
                    if let Some(start_line) = block_stack.pop() {
                        let end_line = tok.line.saturating_sub(1) as u32;
                        if end_line > start_line {
                            ranges.push(FoldingRange::new(start_line, end_line, None));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Consecutive comment lines and import lines folding
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        // Comment block folding
        if trimmed.starts_with('#') {
            let start = i as u32;
            while i < lines.len() && lines[i].trim().starts_with('#') {
                i += 1;
            }
            let end = i.saturating_sub(1) as u32;
            if end > start {
                ranges.push(FoldingRange::new(start, end, Some("comment")));
            }
            continue;
        }

        // Import block folding
        if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            let start = i as u32;
            while i < lines.len()
                && (lines[i].trim().starts_with("import ") || lines[i].trim().starts_with("from "))
            {
                i += 1;
            }
            let end = i.saturating_sub(1) as u32;
            if end > start {
                ranges.push(FoldingRange::new(start, end, Some("imports")));
            }
            continue;
        }

        i += 1;
    }

    ranges
}

/// Discovers all reference locations of the given word within source.
pub fn find_references_for_word(source: &str, word: &str, uri: &str) -> Vec<Location> {
    let mut locs = Vec::new();
    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(_) => return locs,
    };

    for tok in &tokens {
        let matches = match &tok.token_type {
            TokenType::Identifier(id) => id == word,
            TokenType::Assert => word == "assert",
            TokenType::Test => word == "test",
            TokenType::Bench => word == "bench",
            TokenType::Say => word == "say",
            _ => false,
        };

        if matches {
            let start_line = tok.line.saturating_sub(1) as u32;
            let start_col = tok.column.saturating_sub(1) as u32;
            let end_col = start_col + word.len() as u32;
            let range = Range::new(
                Position::new(start_line, start_col),
                Position::new(start_line, end_col),
            );
            locs.push(Location::new(uri.to_string(), range));
        }
    }

    locs
}

/// Formats the document in-memory using the native Alya compiler formatter.
pub fn format_document(source: &str) -> Option<String> {
    match crate::tools::fmt::format_source(source) {
        Ok(formatted) => {
            if formatted != source {
                Some(formatted)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

pub fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "function"
            | "fn"
            | "struct"
            | "enum"
            | "interface"
            | "let"
            | "const"
            | "if"
            | "else"
            | "elif"
            | "while"
            | "for"
            | "repeat"
            | "in"
            | "when"
            | "is"
            | "as"
            | "spawn"
            | "select"
            | "defer"
            | "try"
            | "catch"
            | "finally"
            | "throw"
            | "return"
            | "break"
            | "continue"
            | "say"
            | "pub"
            | "test"
            | "bench"
            | "import"
            | "from"
            | "weak"
            | "true"
            | "false"
            | "null"
            | "and"
            | "or"
            | "not"
            | "end"
    )
}

type FunctionSignatureMap =
    HashMap<String, (Vec<ParameterInformation>, Option<String>, Option<String>)>;

fn extract_functions(source: &str) -> FunctionSignatureMap {
    let mut map = HashMap::new();

    // 1. Built-in functions
    map.insert(
        "say".to_string(),
        (
            vec![ParameterInformation::new("value: any")],
            Some("void".to_string()),
            Some("Outputs formatted expression value to standard output.".to_string()),
        ),
    );
    map.insert(
        "assert".to_string(),
        (
            vec![
                ParameterInformation::new("condition: bool"),
                ParameterInformation::new("message: string = \"\""),
            ],
            Some("void".to_string()),
            Some("Asserts that condition evaluates to true, aborting otherwise.".to_string()),
        ),
    );
    map.insert(
        "len".to_string(),
        (
            vec![ParameterInformation::new("collection: any")],
            Some("int".to_string()),
            Some("Returns number of elements in array, string, or map.".to_string()),
        ),
    );
    map.insert(
        "push".to_string(),
        (
            vec![
                ParameterInformation::new("array: [any]"),
                ParameterInformation::new("item: any"),
            ],
            Some("void".to_string()),
            Some("Appends an element to the end of a dynamic array.".to_string()),
        ),
    );
    map.insert(
        "pop".to_string(),
        (
            vec![ParameterInformation::new("array: [any]")],
            Some("any".to_string()),
            Some("Removes and returns the last element of a dynamic array.".to_string()),
        ),
    );
    map.insert(
        "panic".to_string(),
        (
            vec![ParameterInformation::new("message: string")],
            Some("void".to_string()),
            Some("Terminates process execution immediately with an error message.".to_string()),
        ),
    );

    // 2. Try parsing AST
    let mut lexer = Lexer::new(source);
    if let Ok(tokens) = lexer.tokenize() {
        let mut parser = Parser::new(tokens);
        if let Ok(ast) = parser.parse() {
            for stmt in &ast.statements {
                if let Stmt::Function {
                    name,
                    params,
                    param_types,
                    return_type,
                    defaults,
                    ..
                } = stmt.inner_stmt()
                {
                    let mut p_infos = Vec::new();
                    for (i, p_name) in params.iter().enumerate() {
                        let mut label = p_name.clone();
                        if let Some(Some(t)) = param_types.get(i) {
                            label.push_str(": ");
                            label.push_str(t);
                        }
                        if let Some(Some(_)) = defaults.get(i) {
                            label.push_str(" = ...");
                        }
                        p_infos.push(ParameterInformation::new(&label));
                    }
                    map.insert(name.clone(), (p_infos, return_type.clone(), None));
                }
            }
        }
    }

    // 3. Line scan fallback (for incomplete files during live typing)
    let lines: Vec<&str> = source.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        let mut trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("pub ") {
            trimmed = rest.trim_start();
        }
        if let Some(rest) = trimmed.strip_prefix("function ") {
            let rest = rest.trim_start();
            if let Some(paren_idx) = rest.find('(') {
                let fn_name = rest[..paren_idx].trim().to_string();
                if !fn_name.is_empty() && !map.contains_key(&fn_name) {
                    let after_paren = &rest[paren_idx + 1..];
                    let (args_str, ret_type) = if let Some(close_idx) = after_paren.find(')') {
                        let a = &after_paren[..close_idx];
                        let r = after_paren[close_idx + 1..].trim();
                        let ret = r.strip_prefix("->").map(|arrow| arrow.trim().to_string());
                        (a, ret)
                    } else {
                        (after_paren, None)
                    };

                    let p_infos = if args_str.trim().is_empty() {
                        Vec::new()
                    } else {
                        args_str
                            .split(',')
                            .map(|part| ParameterInformation::new(part.trim()))
                            .collect()
                    };

                    // Collect preceding doc comments
                    let mut doc_lines = Vec::new();
                    let mut back = idx;
                    while back > 0 {
                        back -= 1;
                        let prev = lines[back].trim();
                        if prev.starts_with('#') {
                            let doc_text = prev.trim_start_matches('#').trim();
                            doc_lines.push(doc_text);
                        } else {
                            break;
                        }
                    }
                    doc_lines.reverse();
                    let doc = if doc_lines.is_empty() {
                        None
                    } else {
                        Some(doc_lines.join("\n"))
                    };

                    map.insert(fn_name, (p_infos, ret_type, doc));
                }
            }
        }
    }

    map
}

/// Computes active function signature and parameter context for signature help.
pub fn get_signature_help(source: &str, pos: &Position) -> Option<SignatureHelp> {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = pos.line as usize;
    if line_idx >= lines.len() {
        return None;
    }

    let mut text_up_to_pos = String::new();
    for line in lines.iter().take(line_idx) {
        text_up_to_pos.push_str(line);
        text_up_to_pos.push('\n');
    }
    let cur_line = lines[line_idx];
    let col = (pos.character as usize).min(cur_line.len());
    text_up_to_pos.push_str(&cur_line[..col]);

    let chars: Vec<char> = text_up_to_pos.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut paren_depth = 0;
    let mut comma_count = 0;
    let mut open_paren_idx = None;

    let mut i = chars.len();
    while i > 0 {
        i -= 1;
        let c = chars[i];
        match c {
            ')' => paren_depth += 1,
            '(' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                } else {
                    open_paren_idx = Some(i);
                    break;
                }
            }
            ',' if paren_depth == 0 => {
                comma_count += 1;
            }
            _ => {}
        }
    }

    let p_idx = open_paren_idx?;
    let mut ident_end = p_idx;
    while ident_end > 0 && chars[ident_end - 1].is_whitespace() {
        ident_end -= 1;
    }
    if ident_end == 0 {
        return None;
    }

    let mut ident_start = ident_end;
    while ident_start > 0 {
        let c = chars[ident_start - 1];
        if c.is_alphanumeric() || c == '_' || c == '.' {
            ident_start -= 1;
        } else {
            break;
        }
    }

    if ident_start >= ident_end {
        return None;
    }

    let callee: String = chars[ident_start..ident_end].iter().collect();
    if is_keyword(&callee) {
        return None;
    }

    let fn_map = extract_functions(source);
    let (params, ret_type, doc) = fn_map.get(&callee).or_else(|| {
        let simple_name = callee.split('.').next_back().unwrap_or(&callee);
        fn_map.get(simple_name)
    })?;

    let param_labels: Vec<String> = params.iter().map(|p| p.label.clone()).collect();
    let ret_str = ret_type.as_deref().unwrap_or("void");
    let sig_label = format!("{}({}) -> {}", callee, param_labels.join(", "), ret_str);

    let active_param = comma_count as u32;

    let sig_info = SignatureInformation {
        label: sig_label,
        documentation: doc.clone(),
        parameters: params.clone(),
        active_parameter: Some(active_param),
    };

    Some(SignatureHelp {
        signatures: vec![sig_info],
        active_signature: 0,
        active_parameter: active_param,
    })
}

/// Prepares rename verification by returning symbol range at cursor.
pub fn prepare_rename(source: &str, pos: &Position) -> Option<Range> {
    let word = get_word_at_pos(source, pos)?;
    if is_keyword(&word) || word.chars().next().map_or(true, |c| c.is_ascii_digit()) {
        return None;
    }

    let line = source.lines().nth(pos.line as usize)?;
    let col = (pos.character as usize).min(line.len());
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let target_idx = col.min(chars.len().saturating_sub(1));
    if !is_ident_char(chars[target_idx]) {
        return None;
    }

    let mut start = target_idx;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = target_idx;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }

    Some(Range::new(
        Position::new(pos.line, start as u32),
        Position::new(pos.line, end as u32),
    ))
}

/// Transactionally renames symbol across all open documents.
pub fn rename_symbol(
    documents: &HashMap<String, String>,
    uri: &str,
    pos: &Position,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    if new_name.is_empty()
        || is_keyword(new_name)
        || !new_name
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
        || !new_name.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return None;
    }

    let source = documents.get(uri)?;
    let word = get_word_at_pos(source, pos)?;
    if is_keyword(&word) {
        return None;
    }

    let mut edit = WorkspaceEdit::new();
    for (doc_uri, doc_source) in documents {
        let refs = find_references_for_word(doc_source, &word, doc_uri);
        if !refs.is_empty() {
            let edits: Vec<TextEdit> = refs
                .into_iter()
                .map(|loc| TextEdit {
                    range: loc.range,
                    new_text: new_name.to_string(),
                })
                .collect();
            edit.changes.insert(doc_uri.clone(), edits);
        }
    }

    Some(edit)
}

/// Discovers inlay type hints and parameter name hints.
pub fn get_inlay_hints(source: &str) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(_) => return hints,
    };

    let fn_map = extract_functions(source);

    let mut i = 0;
    while i < tokens.len() {
        // 1. Inlay Type Hints: let x = 42 -> let x: int = 42
        if matches!(tokens[i].token_type, TokenType::Let | TokenType::Const) && i + 2 < tokens.len()
        {
            if let TokenType::Identifier(ref var_name) = tokens[i + 1].token_type {
                if !matches!(tokens[i + 2].token_type, TokenType::Colon)
                    && matches!(tokens[i + 2].token_type, TokenType::Assign)
                    && i + 3 < tokens.len()
                {
                    let inferred = match &tokens[i + 3].token_type {
                        TokenType::Number(_) => Some(": int"),
                        TokenType::Float(_) => Some(": float"),
                        TokenType::String(_) => Some(": string"),
                        TokenType::True | TokenType::False => Some(": bool"),
                        TokenType::LeftBracket => Some(": [any]"),
                        TokenType::Identifier(id) => {
                            if i + 4 < tokens.len()
                                && matches!(tokens[i + 4].token_type, TokenType::LeftBrace)
                            {
                                Some(id.as_str())
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };

                    if let Some(t_label) = inferred {
                        let label_str = if t_label.starts_with(':') {
                            t_label.to_string()
                        } else {
                            format!(": {}", t_label)
                        };
                        let line = tokens[i + 1].line.saturating_sub(1) as u32;
                        let col =
                            tokens[i + 1].column.saturating_sub(1) as u32 + var_name.len() as u32;

                        hints.push(InlayHint {
                            position: Position::new(line, col),
                            label: label_str,
                            kind: Some(InlayHintKind::Type),
                            padding_left: false,
                            padding_right: true,
                        });
                    }
                }
            }
        }

        // 2. Inlay Parameter Hints: call(factor: 2.5)
        if let TokenType::Identifier(ref callee_name) = tokens[i].token_type {
            if i + 1 < tokens.len() && matches!(tokens[i + 1].token_type, TokenType::LeftParen) {
                if let Some((params, _, _)) = fn_map.get(callee_name) {
                    if !params.is_empty() {
                        let mut param_idx = 0;
                        let mut scan = i + 2;
                        let mut paren_nest = 1;
                        let mut at_arg_start = true;

                        while scan < tokens.len() && paren_nest > 0 {
                            match &tokens[scan].token_type {
                                TokenType::LeftParen
                                | TokenType::LeftBracket
                                | TokenType::LeftBrace => {
                                    paren_nest += 1;
                                    at_arg_start = false;
                                }
                                TokenType::RightParen => {
                                    paren_nest -= 1;
                                    at_arg_start = false;
                                }
                                TokenType::RightBracket | TokenType::RightBrace => {
                                    paren_nest -= 1;
                                    at_arg_start = false;
                                }
                                TokenType::Comma => {
                                    if paren_nest == 1 {
                                        param_idx += 1;
                                        at_arg_start = true;
                                    }
                                }
                                _ => {
                                    if at_arg_start && paren_nest == 1 {
                                        if let Some(p_info) = params.get(param_idx) {
                                            let p_name = p_info
                                                .label
                                                .split(':')
                                                .next()
                                                .unwrap_or(&p_info.label)
                                                .trim();

                                            let is_same_name = match &tokens[scan].token_type {
                                                TokenType::Identifier(id) => id == p_name,
                                                _ => false,
                                            };

                                            if !is_same_name && !p_name.is_empty() {
                                                let line =
                                                    tokens[scan].line.saturating_sub(1) as u32;
                                                let col =
                                                    tokens[scan].column.saturating_sub(1) as u32;
                                                hints.push(InlayHint {
                                                    position: Position::new(line, col),
                                                    label: format!("{}:", p_name),
                                                    kind: Some(InlayHintKind::Parameter),
                                                    padding_left: false,
                                                    padding_right: true,
                                                });
                                            }
                                        }
                                        at_arg_start = false;
                                    }
                                }
                            }
                            scan += 1;
                        }
                    }
                }
            }
        }

        i += 1;
    }

    hints
}

fn token_length(tok: &Token, line_str: Option<&str>) -> usize {
    match &tok.token_type {
        TokenType::Identifier(id) => id.len(),
        TokenType::String(s) => s.len() + 2,
        TokenType::Rune(_) => 3,
        TokenType::Function => 8,
        TokenType::Struct => 6,
        TokenType::Enum => 4,
        TokenType::Interface => 9,
        TokenType::If => 2,
        TokenType::Else => 4,
        TokenType::Elif => 4,
        TokenType::While => 5,
        TokenType::For => 3,
        TokenType::Repeat => 6,
        TokenType::When => 4,
        TokenType::Try => 3,
        TokenType::Catch => 5,
        TokenType::Finally => 7,
        TokenType::Throw => 5,
        TokenType::Return => 6,
        TokenType::Break => 5,
        TokenType::Continue => 8,
        TokenType::Say => 3,
        TokenType::Let => 3,
        TokenType::Const => 5,
        TokenType::Pub => 3,
        TokenType::Test => 4,
        TokenType::Bench => 5,
        TokenType::Import => 6,
        TokenType::From => 4,
        TokenType::Weak => 4,
        TokenType::Spawn => 5,
        TokenType::Select => 6,
        TokenType::Defer => 5,
        TokenType::End => 3,
        TokenType::Is => 2,
        TokenType::As => 2,
        TokenType::In => 2,
        TokenType::True => 4,
        TokenType::False => 5,
        TokenType::Null => 4,
        TokenType::Arrow | TokenType::FatArrow => 2,
        TokenType::Equal | TokenType::NotEqual | TokenType::LessEqual | TokenType::GreaterEqual => {
            2
        }
        TokenType::PlusAssign
        | TokenType::MinusAssign
        | TokenType::MultiplyAssign
        | TokenType::DivideAssign
        | TokenType::ModuloAssign => 2,
        TokenType::ColonColon
        | TokenType::QuestionDot
        | TokenType::NullCoalesce
        | TokenType::DotDot => 2,
        TokenType::DotDotDot => 3,
        TokenType::Plus
        | TokenType::Minus
        | TokenType::Multiply
        | TokenType::Divide
        | TokenType::Modulo => 1,
        TokenType::Assign | TokenType::Less | TokenType::Greater | TokenType::Not => 1,
        TokenType::Colon | TokenType::Comma | TokenType::Dot | TokenType::Question => 1,
        TokenType::Number(_) | TokenType::Float(_) => {
            if let Some(l) = line_str {
                let start = tok.column.saturating_sub(1);
                if start < l.len() {
                    let len = l[start..]
                        .chars()
                        .take_while(|c| {
                            c.is_ascii_digit()
                                || *c == '.'
                                || *c == 'e'
                                || *c == 'E'
                                || *c == '_'
                                || *c == 'x'
                                || *c == 'o'
                                || *c == 'b'
                        })
                        .count();
                    if len > 0 {
                        len
                    } else {
                        1
                    }
                } else {
                    1
                }
            } else {
                1
            }
        }
        _ => 1,
    }
}

/// Declaration name sets collected from the file AST, shared by semantic
/// classification of identifiers and f-string interpolation contents.
struct SemanticNameSets<'a> {
    struct_names: &'a HashSet<String>,
    enum_names: &'a HashSet<String>,
    interface_names: &'a HashSet<String>,
    function_names: &'a HashSet<String>,
    variant_names: &'a HashSet<String>,
    field_names: &'a HashSet<String>,
    param_names: &'a HashSet<String>,
    const_names: &'a HashSet<String>,
    /// Import aliases (`import "p" as a`): namespace qualifiers.
    aliases: &'a HashSet<String>,
    /// Names the compiler merges into scope from imports, with the token
    /// kind of their declaration (`from` symbols transitively, plain
    /// unaliased imports single-level). File-locals shadow these.
    scope_kinds: &'a HashMap<String, (u32, u32)>,
}

/// Known-kind classification: primitives, file-local declarations, import
/// aliases (namespaces), then imported-scope kinds. `None` means unknown
/// (plain variable fallback, or function when called).
fn classify_known_ident(name: &str, sets: &SemanticNameSets) -> Option<(u32, u32)> {
    match name {
        "int" | "float" | "string" | "bool" | "void" | "any" | "byte" | "char" | "Fiber"
        | "Channel" | "Mutex" | "WaitGroup" | "f64x4" | "f32x8" | "i32x8" | "i64x4" | "Tensor" => {
            Some((0, 8))
        }
        _ if sets.struct_names.contains(name) => Some((4, 0)),
        _ if sets.enum_names.contains(name) => Some((2, 0)),
        _ if sets.interface_names.contains(name) => Some((3, 0)),
        _ if sets.function_names.contains(name) => Some((10, 0)),
        _ if sets.variant_names.contains(name) => Some((9, 0)),
        _ if sets.field_names.contains(name) => Some((8, 0)),
        _ if sets.param_names.contains(name) => Some((6, 0)),
        _ if sets.const_names.contains(name) => Some((7, 4)),
        _ if sets.aliases.contains(name) => Some((17, 0)),
        _ => sets.scope_kinds.get(name).copied(),
    }
}

/// Maps a declaration kind to its semantic token type for imported names.
fn symbol_kind_token(kind: SymbolKind) -> (u32, u32) {
    match kind {
        SymbolKind::Function => (10, 0),
        SymbolKind::Struct => (4, 0),
        SymbolKind::Enum => (2, 0),
        SymbolKind::Interface => (3, 0),
        SymbolKind::Let => (7, 0),
    }
}

/// Enumerates top-level declared symbols of a module source. When the
/// module declares any `pub` item, only `pub` items merge into importers
/// (mirroring the compiler); otherwise everything is visible.
fn declared_symbols_in_source(source: &str) -> Vec<(String, SymbolKind)> {
    struct Raw {
        name: String,
        kind: SymbolKind,
        is_pub: bool,
    }
    let mut all: Vec<Raw> = Vec::new();
    for line in source.lines() {
        let mut trimmed = line.trim_start();
        let is_pub = if let Some(rest) = trimmed.strip_prefix("pub ") {
            trimmed = rest.trim_start();
            true
        } else {
            false
        };
        let (name, kind) = if let Some(rest) = trimmed.strip_prefix("function ") {
            let head: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            // `Type.method` declares the method, not the head.
            let after: String = rest.trim_start()[head.len()..]
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
                .collect();
            let name = after
                .rsplit('.')
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or(&head)
                .to_string();
            (name, SymbolKind::Function)
        } else if let Some(rest) = trimmed.strip_prefix("struct ") {
            let n: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (n, SymbolKind::Struct)
        } else if let Some(rest) = trimmed.strip_prefix("enum ") {
            let n: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (n, SymbolKind::Enum)
        } else if let Some(rest) = trimmed.strip_prefix("interface ") {
            let n: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (n, SymbolKind::Interface)
        } else if let Some(rest) = trimmed
            .strip_prefix("let ")
            .or_else(|| trimmed.strip_prefix("const "))
        {
            let n: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (n, SymbolKind::Let)
        } else {
            continue;
        };
        if !name.is_empty() {
            all.push(Raw { name, kind, is_pub });
        }
    }
    let gated = all.iter().any(|r| r.is_pub);
    all.into_iter()
        .filter(|r| !gated || r.is_pub)
        .map(|r| (r.name, r.kind))
        .collect()
}

/// One identifier found inside a string interpolation span, with its
/// absolute column on the line and whether it is called (`name(`).
struct FstringIdent {
    col: u32,
    len: u32,
    name: String,
    is_call: bool,
}

/// Scans one source line of a string literal for interpolation contents,
/// starting at byte offset `from` (just past the opening quotes). Tracks
/// `{{` / `}}` escapes, nested quotes, and brace depth; stops at the
/// closing quote or end of line (later lines of multiline literals are
/// skipped). Keywords and digit-led fragments (`{x:.2f}`) are not usages.
fn scan_fstring_line(line: &str, from: usize) -> Vec<FstringIdent> {
    let b = line.as_bytes();
    let mut i = from.min(b.len());
    let mut depth = 0u32;
    let mut out = Vec::new();
    // Byte offset -> char column (UTF-16-safe for BMP text).
    let col_of = |idx: usize| line[..idx.min(line.len())].chars().count() as u32;
    while i < b.len() {
        let c = b[i];
        if depth == 0 {
            if c == b'"' {
                break;
            }
            if c == b'{' {
                if i + 1 < b.len() && b[i + 1] == b'{' {
                    i += 2;
                    continue;
                }
                depth = 1;
            }
            i += 1;
            continue;
        }
        if c == b'"' || c == b'\'' {
            let q = c;
            i += 1;
            while i < b.len() && b[i] != q {
                if b[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if c == b'{' {
            depth += 1;
            i += 1;
            continue;
        }
        if c == b'}' {
            depth -= 1;
            i += 1;
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            if i > 0 && b[i - 1].is_ascii_digit() {
                // Digit-led fragment (`{x:.2f}`): skip the whole run.
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                continue;
            }
            let mut j = i + 1;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                j += 1;
            }
            let name = line[i..j].to_string();
            if !is_keyword(&name) {
                let mut k = j;
                while k < b.len() && (b[k] == b' ' || b[k] == b'\t') {
                    k += 1;
                }
                out.push(FstringIdent {
                    col: col_of(i),
                    len: (j - i) as u32,
                    name,
                    is_call: k < b.len() && b[k] == b'(',
                });
            }
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

/// Discovers AST-driven semantic token classification and delta encoding.
///
/// `file_dir` (the document's directory) lets imported scopes resolve for
/// relative paths; without it only cwd-absolute targets (e.g. stdlib next
/// to the server cwd) and the current file classify.
pub fn get_semantic_tokens(source: &str, file_dir: Option<&std::path::Path>) -> SemanticTokens {
    let mut raw_tokens = Vec::new();
    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(_) => return SemanticTokens { data: Vec::new() },
    };

    let mut struct_names = HashSet::new();
    let mut enum_names = HashSet::new();
    let mut interface_names = HashSet::new();
    let mut function_names = HashSet::new();
    let mut variant_names = HashSet::new();
    let mut field_names = HashSet::new();
    let mut param_names = HashSet::new();
    let mut const_names = HashSet::new();

    let mut parser = Parser::new(tokens.clone());
    if let Ok(ast) = parser.parse() {
        for stmt in &ast.statements {
            match stmt.inner_stmt() {
                Stmt::StructDef { name, fields, .. } => {
                    struct_names.insert(name.clone());
                    for f in fields {
                        field_names.insert(f.clone());
                    }
                }
                Stmt::EnumDef { name, variants } => {
                    enum_names.insert(name.clone());
                    for (v, _) in variants {
                        variant_names.insert(v.clone());
                    }
                }
                Stmt::InterfaceDef { name, methods, .. } => {
                    interface_names.insert(name.clone());
                    for m in methods {
                        function_names.insert(m.name.clone());
                    }
                }
                Stmt::Function { name, params, .. } => {
                    function_names.insert(name.clone());
                    for p in params {
                        param_names.insert(p.clone());
                    }
                }
                Stmt::Const { name, .. } => {
                    const_names.insert(name.clone());
                }
                _ => {}
            }
        }
    }

    let source_lines: Vec<&str> = source.lines().collect();

    // Import scope: aliases act as namespaces; `from` symbols and plain
    // unaliased imports contribute their declaration kinds (single-level
    // for plain imports, transitive for explicit `from` names).
    let file_imports = parse_file_imports(source);
    let mut aliases = HashSet::new();
    for imp in &file_imports {
        if let Some(a) = &imp.alias {
            aliases.insert(a.clone());
        }
    }
    let mut scope_kinds: HashMap<String, (u32, u32)> = HashMap::new();
    if file_imports
        .iter()
        .any(|im| !im.symbols.is_empty() || (!im.is_from && im.alias.is_none()))
    {
        let mut visited = HashSet::new();
        for imp in &file_imports {
            let target = match resolve_import_to_file(&imp.path, file_dir) {
                Some(t) => t,
                None => continue,
            };
            if imp.is_from {
                for sym in &imp.symbols {
                    let eff = sym.alias.as_ref().unwrap_or(&sym.name).clone();
                    if scope_kinds.contains_key(&eff) {
                        continue;
                    }
                    if let Some((_, _, kind)) = find_first_seg(&target, &sym.name, 0, &mut visited)
                    {
                        scope_kinds.insert(eff, symbol_kind_token(kind));
                    }
                }
            } else if imp.alias.is_none() {
                if let Ok(target_src) = std::fs::read_to_string(&target) {
                    for (name, kind) in declared_symbols_in_source(&target_src) {
                        scope_kinds.entry(name).or_insert(symbol_kind_token(kind));
                    }
                }
            }
        }
    }
    let name_sets = SemanticNameSets {
        struct_names: &struct_names,
        enum_names: &enum_names,
        interface_names: &interface_names,
        function_names: &function_names,
        variant_names: &variant_names,
        field_names: &field_names,
        param_names: &param_names,
        const_names: &const_names,
        aliases: &aliases,
        scope_kinds: &scope_kinds,
    };

    for i in 0..tokens.len() {
        let tok = &tokens[i];
        let line_0 = tok.line.saturating_sub(1) as u32;
        let col_0 = tok.column.saturating_sub(1) as u32;
        let cur_line = source_lines.get(line_0 as usize).copied();

        // Interpolated spans: every string kind (`f"`, `"`, `"""`, `r"`,
        // backtick) interpolates at runtime, so the blanket string token
        // is dropped and interpolation contents get their real kinds.
        // TextMate paints the literal parts.
        if let TokenType::String(value) = &tok.token_type {
            if value.as_bytes().contains(&b'{') {
                if let Some(line_text) = cur_line {
                    // Past optional `f`/`r`/`b` prefixes and opening quotes.
                    let lb = line_text.as_bytes();
                    let mut from = col_0 as usize;
                    while from < lb.len() && lb[from].is_ascii_alphabetic() {
                        from += 1;
                    }
                    let mut quotes = 0;
                    while from < lb.len() && lb[from] == b'"' && quotes < 3 {
                        from += 1;
                        quotes += 1;
                    }
                    if quotes == 0 {
                        // Backtick raw string (or untraceable opening).
                        while from < lb.len() && lb[from] != b'`' {
                            from += 1;
                        }
                        from += 1;
                    }
                    for ident in scan_fstring_line(line_text, from) {
                        let (tt, mods) = classify_known_ident(&ident.name, &name_sets)
                            .unwrap_or(if ident.is_call { (10, 0) } else { (7, 0) });
                        raw_tokens.push(RawSemanticToken {
                            line: line_0,
                            start_col: ident.col,
                            length: ident.len,
                            token_type: tt,
                            token_modifiers: mods,
                        });
                    }
                    continue;
                }
            }
        }

        let (token_type, token_modifiers, length) = match &tok.token_type {
            TokenType::Function
            | TokenType::Struct
            | TokenType::Enum
            | TokenType::Interface
            | TokenType::If
            | TokenType::Else
            | TokenType::Elif
            | TokenType::While
            | TokenType::For
            | TokenType::Repeat
            | TokenType::When
            | TokenType::Try
            | TokenType::Catch
            | TokenType::Finally
            | TokenType::Throw
            | TokenType::Return
            | TokenType::Break
            | TokenType::Continue
            | TokenType::Say
            | TokenType::Let
            | TokenType::Const
            | TokenType::Pub
            | TokenType::Test
            | TokenType::Bench
            | TokenType::Import
            | TokenType::From
            | TokenType::Weak
            | TokenType::Spawn
            | TokenType::Select
            | TokenType::Defer
            | TokenType::End
            | TokenType::Is
            | TokenType::As
            | TokenType::In
            | TokenType::True
            | TokenType::False
            | TokenType::Null => (12, 0, token_length(tok, cur_line)),

            TokenType::Number(_) | TokenType::Float(_) => (15, 0, token_length(tok, cur_line)),
            TokenType::String(_) | TokenType::Rune(_) => (14, 0, token_length(tok, cur_line)),

            TokenType::Plus
            | TokenType::Minus
            | TokenType::Multiply
            | TokenType::Divide
            | TokenType::Modulo
            | TokenType::Assign
            | TokenType::Equal
            | TokenType::NotEqual
            | TokenType::Less
            | TokenType::LessEqual
            | TokenType::Greater
            | TokenType::GreaterEqual
            | TokenType::Not
            | TokenType::Arrow
            | TokenType::FatArrow
            | TokenType::PlusAssign
            | TokenType::MinusAssign
            | TokenType::MultiplyAssign
            | TokenType::DivideAssign
            | TokenType::ModuloAssign => (16, 0, token_length(tok, cur_line)),

            TokenType::Identifier(name) => {
                let prev_tok = if i > 0 { Some(&tokens[i - 1]) } else { None };
                let (tt, mods) = match prev_tok.map(|p| &p.token_type) {
                    Some(TokenType::Function) => (10, 1 | 2),
                    Some(TokenType::Struct) => (4, 1 | 2),
                    Some(TokenType::Enum) => (2, 1 | 2),
                    Some(TokenType::Interface) => (3, 1 | 2),
                    Some(TokenType::Const) => (7, 1 | 4),
                    _ => classify_known_ident(name, &name_sets).unwrap_or_else(|| {
                        // Unknown `name(` is a call: mirrors the TextMate
                        // function-call rule so both layers agree.
                        let is_call = matches!(
                            tokens.get(i + 1).map(|t| &t.token_type),
                            Some(TokenType::LeftParen)
                        );
                        if is_call {
                            (10, 0)
                        } else {
                            (7, 0)
                        }
                    }),
                };
                (tt, mods, name.len())
            }
            _ => continue,
        };

        if length > 0 {
            raw_tokens.push(RawSemanticToken {
                line: line_0,
                start_col: col_0,
                length: length as u32,
                token_type,
                token_modifiers,
            });
        }
    }

    SemanticTokens::from_raw_tokens(raw_tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_tokens_fstring_interpolation() {
        // Import-aware kinds: `floor` resolves through the `from` import to
        // a function, `text_ops` is a namespace alias, unknown calls fall
        // back to function, plain unknown words to variable. The point is
        // interpolation matches bare usage, with no blanket string token
        // covering the spans.
        let src = "from \"std/math\" import floor, nosuchfn as nf\nimport \"std/str\" as text_ops\nsay f\"v: {floor(7.9)} q: {text_ops} r: {nf(1)} s: {nosuchfn()} t: {plainvar}\"\nsay \"plain\"\n";
        let toks = get_semantic_tokens(src, None);
        assert_eq!(toks.data.len() % 5, 0);
        let mut decoded: Vec<(u32, u32, u32, u32, u32)> = Vec::new();
        let mut line = 0u32;
        let mut col = 0u32;
        for chunk in toks.data.chunks(5) {
            line += chunk[0];
            if chunk[0] == 0 {
                col += chunk[1];
            } else {
                col = chunk[1];
            }
            decoded.push((line, col, chunk[2], chunk[3], chunk[4]));
        }
        assert!(
            decoded.contains(&(2, 10, 5, 10, 0)),
            "from-imported floor call: {:?}",
            decoded
        );
        assert!(
            decoded.contains(&(2, 26, 8, 17, 0)),
            "alias qualifier is namespace: {:?}",
            decoded
        );
        assert!(
            decoded.contains(&(2, 40, 2, 10, 0)),
            "unresolved alias call: {:?}",
            decoded
        );
        assert!(
            decoded.contains(&(2, 51, 8, 10, 0)),
            "unknown call: {:?}",
            decoded
        );
        assert!(
            decoded.contains(&(2, 67, 8, 7, 0)),
            "plain unknown word: {:?}",
            decoded
        );
        // No string token touches line 2 (the f-string line); the plain
        // string on line 3 keeps its blanket token.
        assert!(
            !decoded.iter().any(|&(l, _, _, t, _)| l == 2 && t == 14),
            "no blanket string on f-string line: {:?}",
            decoded
        );
        assert!(
            decoded.iter().any(|&(l, _, _, t, _)| l == 3 && t == 14),
            "plain string keeps token: {:?}",
            decoded
        );
    }

    #[test]
    fn test_semantic_tokens_plain_string_interpolation() {
        // Plain strings interpolate at runtime too: `{floor}` gets the
        // from-imported function kind, not string color.
        let src = "from \"std/math\" import floor\nsay \"v: {floor} done\"\n";
        let toks = get_semantic_tokens(src, None);
        let mut decoded: Vec<(u32, u32, u32, u32, u32)> = Vec::new();
        let mut line = 0u32;
        let mut col = 0u32;
        for chunk in toks.data.chunks(5) {
            line += chunk[0];
            if chunk[0] == 0 {
                col += chunk[1];
            } else {
                col = chunk[1];
            }
            decoded.push((line, col, chunk[2], chunk[3], chunk[4]));
        }
        // `say "v: {floor} done"`: `{` at col 8, `floor` at 9..14.
        assert!(
            decoded.contains(&(1, 9, 5, 10, 0)),
            "plain-string interpolation: {:?}",
            decoded
        );
        assert!(
            !decoded.iter().any(|&(l, _, _, t, _)| l == 1 && t == 14),
            "no blanket string: {:?}",
            decoded
        );
    }

    #[test]
    fn test_resolve_import_to_file_returns_absolute_paths() {
        // Temp dir without a local stdlib: `std/*` can only resolve via
        // the process cwd (the crate dir, which ships `stdlib/`). The
        // result must still be absolute — relative results escape as
        // bogus `file://` URIs downstream.
        let dir = std::env::temp_dir().join(format!("alya-lsp-abs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let hit = resolve_import_to_file("std/str", Some(&dir))
            .expect("crate stdlib resolves via process cwd");
        assert!(hit.is_absolute(), "must be absolute, got {}", hit.display());
        assert!(hit.exists());
        // Same guarantee for plain relative imports.
        std::fs::write(dir.join("a.alya"), "say 1\n").unwrap();
        let rel = resolve_import_to_file("./a.alya", Some(&dir)).expect("relative resolves");
        assert!(rel.is_absolute(), "must be absolute, got {}", rel.display());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
