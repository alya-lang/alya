use super::protocol::{
    CompletionItem, Diagnostic, DocumentSymbol, FoldingRange, Location, Position, Range,
};
use crate::ast::Stmt;
use crate::lexer::{Lexer, Token, TokenType};
use crate::parser::Parser;
use std::collections::HashSet;

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
