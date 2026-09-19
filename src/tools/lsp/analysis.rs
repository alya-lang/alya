use super::protocol::{CompletionItem, Diagnostic, Position, Range};
use crate::ast::Stmt;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::collections::HashSet;

pub fn check_document(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(err) => {
            diagnostics.push(parse_error_to_diagnostic(&err, source));
            return diagnostics;
        }
    };

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
            let lint_diags = crate::tools::lint::run_all_rules(
                &program,
                &tokens,
                std::path::Path::new("document.alya"),
            );
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

fn get_word_at_pos(source: &str, pos: &Position) -> Option<String> {
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
