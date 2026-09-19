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

            // 2. Linter Analysis Rules
            let lint_diags = crate::tools::lint::run_all_rules(&program, &tokens, target_path);
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
        "f32x8" => Some("**struct f32x8**: 256-bit hardware SIMD vector containing 8x 32-bit single-precision floating-point numbers (`std/simd`).\n\nSupports parallel arithmetic and horizontal sum reduction."),
        "i32x8" => Some("**struct i32x8**: 256-bit hardware SIMD vector containing 8x 32-bit signed integers (`std/simd`).\n\nSupports 8-lane parallel integer addition."),
        "i64x4" => Some("**struct i64x4**: 256-bit hardware SIMD vector containing 4x 64-bit signed integers (`std/simd`).\n\nSupports 4-lane parallel 64-bit integer addition."),
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

pub fn get_definition_pos(source: &str, pos: &Position) -> Option<Position> {
    let word = get_word_at_pos(source, pos)?;

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
                let char_idx = line.find(&word).unwrap_or(0) as u32;
                return Some(Position::new(line_idx as u32, char_idx));
            }
        } else if let Some(rest) = trimmed.strip_prefix("struct ") {
            let rest = rest.trim_start();
            let st_name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if st_name == word {
                let char_idx = line.find(&word).unwrap_or(0) as u32;
                return Some(Position::new(line_idx as u32, char_idx));
            }
        } else if let Some(rest) = trimmed.strip_prefix("interface ") {
            let rest = rest.trim_start();
            let if_name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if if_name == word {
                let char_idx = line.find(&word).unwrap_or(0) as u32;
                return Some(Position::new(line_idx as u32, char_idx));
            }
        } else if let Some(rest) = trimmed.strip_prefix("enum ") {
            let rest = rest.trim_start();
            let en_name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if en_name == word {
                let char_idx = line.find(&word).unwrap_or(0) as u32;
                return Some(Position::new(line_idx as u32, char_idx));
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
                let char_idx = line.find(&word).unwrap_or(0) as u32;
                return Some(Position::new(line_idx as u32, char_idx));
            }
        }
    }

    None
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
                        TokenType::Number(n) => {
                            if n.fract() == 0.0 {
                                Some(": int")
                            } else {
                                Some(": float")
                            }
                        }
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

/// Discovers AST-driven semantic token classification and delta encoding.
pub fn get_semantic_tokens(source: &str) -> SemanticTokens {
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

    for i in 0..tokens.len() {
        let tok = &tokens[i];
        let line_0 = tok.line.saturating_sub(1) as u32;
        let col_0 = tok.column.saturating_sub(1) as u32;
        let cur_line = source_lines.get(line_0 as usize).copied();

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
                    _ => match name.as_str() {
                        "int" | "float" | "string" | "bool" | "void" | "any" | "byte" | "char"
                        | "Fiber" | "Channel" | "Mutex" | "WaitGroup" | "f64x4" | "f32x8"
                        | "i32x8" | "i64x4" | "Tensor" => (0, 8),
                        _ if struct_names.contains(name) => (4, 0),
                        _ if enum_names.contains(name) => (2, 0),
                        _ if interface_names.contains(name) => (3, 0),
                        _ if function_names.contains(name) => (10, 0),
                        _ if variant_names.contains(name) => (9, 0),
                        _ if field_names.contains(name) => (8, 0),
                        _ if param_names.contains(name) => (6, 0),
                        _ if const_names.contains(name) => (7, 4),
                        _ => (7, 0),
                    },
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
