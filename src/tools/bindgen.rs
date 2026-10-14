//! C header bridge: `alya bindgen` (alya-lang/alya#196).
//!
//! Parses a C header SUBSET and emits an `extern "C"` block using only
//! existing syntax (no EBNF change — see Chapter 12 §1.7): function
//! prototypes, `@repr(C)` structs, enums, transparent typedefs, and
//! integer `#define` constants.
//!
//! Soundness rule: anything the mapper cannot prove width-exact
//! becomes a `-- SKIPPED: <reason>` comment, never guessed code.
//! In particular `long` (LLP64 vs LP64), plain `char` (signedness is
//! target-dependent), varargs, unions, and bitfields are all loud
//! skips. Pointer shapes always map (`T*` -> `ptr`, with
//! `const char *` -> `str` matching established package practice).

use std::collections::HashMap;

#[derive(Default)]
pub struct BindgenOptions {
    pub from_lib: Option<String>,
}

/// A parsed top-level C item worth emitting.
enum Item {
    Struct {
        name: String,
        fields: Vec<(String, String)>,
    },
    Enum {
        name: Option<String>,
        variants: Vec<(String, i128)>,
    },
    Const {
        name: String,
        value: String,
    },
    Function {
        name: String,
        params: Vec<(String, String)>,
        ret: Option<String>,
    },
    Skipped {
        reason: String,
        source: String,
    },
}

/// Strip `//` and `/* */` comments, preserving newlines for messages.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_string {
            out.push(c);
            if c == '"' && (i == 0 || bytes[i - 1] as char != '\\') {
                in_string = false;
            }
            i += 1;
        } else if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
        } else if c == '/' && i + 1 < bytes.len() && bytes[i + 1] as char == '/' {
            while i < bytes.len() && bytes[i] as char != '\n' {
                i += 1;
            }
        } else if c == '/' && i + 1 < bytes.len() && bytes[i + 1] as char == '*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] as char == '*' && bytes[i + 1] as char == '/') {
                if bytes[i] as char == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i += 2;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Split into top-level items: preprocessor lines, `;`-terminated
/// declarations (brace blocks attach), and balanced `{...}` groups.
fn split_items(src: &str) -> Vec<String> {
    // Join backslash continuations first (multi-line #defines).
    let mut joined = String::new();
    let mut lines = src.lines().peekable();
    while let Some(line) = lines.next() {
        let mut cur = line.to_string();
        while cur.ends_with('\\') {
            cur.pop();
            cur.push_str(lines.next().unwrap_or(""));
        }
        joined.push_str(&cur);
        joined.push('\n');
    }
    // Drop conditional-structure directives (`#ifdef`/`#ifndef`/`#if`/
    // `#else`/`#elif`/`#endif`, `#include`, `#pragma`, ...): they would
    // otherwise corrupt brace tracking (`extern "C" {` opened in one
    // branch, closed in another). Only `#define` carries bindings.
    // Consequence, documented: both branches of a platform `#if`
    // survive, so conflicting duplicates are dropped by name below.
    let mut filtered = String::new();
    for line in joined.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') && !trimmed.starts_with("#define") {
            continue;
        }
        filtered.push_str(line);
        filtered.push('\n');
    }
    let mut items = Vec::new();
    let mut current = String::new();
    let mut paren = 0i32;
    let mut brace = 0i32;
    let mut bracket = 0i32;
    for line in filtered.lines() {
        let trimmed = line.trim_start();
        // Only `#define` directives survive the pre-pass above.
        if paren == 0 && brace == 0 && bracket == 0 && trimmed.starts_with('#') {
            if !current.trim().is_empty() {
                items.push(current.trim().to_string());
                current = String::new();
            }
            items.push(trimmed.to_string());
            continue;
        }
        for ch in line.chars() {
            match ch {
                '(' => paren += 1,
                ')' => paren -= 1,
                '{' => brace += 1,
                '}' => brace -= 1,
                '[' => bracket += 1,
                ']' => bracket -= 1,
                ';' if paren == 0 && brace == 0 && bracket == 0 => {
                    current.push(';');
                    items.push(current.trim().to_string());
                    current = String::new();
                    continue;
                }
                _ => {}
            }
            current.push(ch);
        }
        current.push('\n');
        // A balanced `{...}` group standing alone (e.g. the body of an
        // `extern "C" { }` wrapper is handled by recursion, but a lone
        // closing `}` at depth 0 ends a wrapper item).
        if brace == 0 && paren == 0 && bracket == 0 && current.trim() == "}" {
            items.push("}".to_string());
            current = String::new();
        }
    }
    if !current.trim().is_empty() {
        items.push(current.trim().to_string());
    }
    items
        .into_iter()
        .filter(|item| !item.trim().is_empty())
        .collect()
}

/// Map one C base type (qualifiers already stripped) to Alya.
/// Returns `None` for platform-dependent widths: those are loud
/// skips, never guesses.
fn map_base_type(words: &[&str]) -> Option<String> {
    let joined = words.join(" ");
    match joined.as_str() {
        "void" => Some("void".to_string()),
        "_Bool" | "bool" => Some("bool".to_string()),
        "char" => None,
        "signed char" => Some("i8".to_string()),
        "unsigned char" => Some("u8".to_string()),
        "short" | "short int" | "signed short" | "signed short int" => Some("i16".to_string()),
        "unsigned short" | "unsigned short int" => Some("u16".to_string()),
        "int" | "signed" | "signed int" => Some("i32".to_string()),
        "unsigned" | "unsigned int" => Some("u32".to_string()),
        "long" | "long int" | "signed long" | "unsigned long" | "unsigned long int" => None,
        "long long" | "long long int" | "signed long long" => Some("i64".to_string()),
        "unsigned long long" | "unsigned long long int" => Some("u64".to_string()),
        "float" => Some("f32".to_string()),
        "double" => Some("f64".to_string()),
        "long double" => None,
        "size_t" => Some("usize".to_string()),
        "ssize_t" | "ptrdiff_t" | "intptr_t" => Some("isize".to_string()),
        "uintptr_t" => Some("usize".to_string()),
        "int8_t" => Some("i8".to_string()),
        "uint8_t" => Some("u8".to_string()),
        "int16_t" => Some("i16".to_string()),
        "uint16_t" => Some("u16".to_string()),
        "int32_t" => Some("i32".to_string()),
        "uint32_t" => Some("u32".to_string()),
        "int64_t" => Some("i64".to_string()),
        "uint64_t" => Some("u64".to_string()),
        _ => None,
    }
}

/// C type keywords: never identifiers, even when they appear last.
fn is_c_type_keyword(word: &str) -> bool {
    matches!(
        word,
        "void"
            | "_Bool"
            | "bool"
            | "char"
            | "signed"
            | "unsigned"
            | "short"
            | "long"
            | "int"
            | "float"
            | "double"
            | "struct"
            | "enum"
            | "union"
    )
}

/// Split a declarator into (base words, pointer depth, name).
/// Handles `const`, `struct`/`enum`/`union` tags, arrays (reported),
/// and function pointers (reported). Returns `Err` with a reason.
fn split_declarator(decl: &str) -> Result<(Vec<String>, usize, String, bool, bool), String> {
    // Function pointer: `ret (*name)(args)` — a word whatever it is
    // called, named or not (an anonymous `void(*)(void*)` destructor
    // still rides one word; the caller synthesizes `argN` for it).
    if let Some(star) = decl.find("(*") {
        let after = &decl[star + 2..];
        if let Some(end) = after.find(')') {
            let name = after[..end].trim().to_string();
            if name.contains(|c: char| !(c.is_alphanumeric() || c == '_')) {
                return Err("malformed function pointer".to_string());
            }
            return Ok((vec!["ptr".to_string()], 0, name, false, false));
        }
        return Err("malformed function pointer".to_string());
    }
    let is_array = decl.contains('[');
    let mut ptr_depth = 0usize;
    let mut words: Vec<String> = Vec::new();
    for tok in decl
        .replace('*', " * ")
        .replace(',', " ")
        .split_whitespace()
    {
        if tok == "*" {
            ptr_depth += 1;
        } else if tok.starts_with('[') || tok == "]" || tok.contains(']') || tok.contains('[') {
            // Array bounds are not identifiers.
        } else {
            words.push(tok.to_string());
        }
    }
    // Drop qualifiers anywhere.
    words.retain(|w| {
        !matches!(
            w.as_str(),
            "const" | "volatile" | "restrict" | "static" | "inline" | "_Noreturn"
        )
    });
    // Trailing identifier is the name (unless the whole thing is a bare type).
    // A lone word is always a type: C has no nameless declarators, and a
    // return position never carries a name (`uword_t f(...)` is a type).
    let mut name = String::new();
    if words.len() > 1 {
        if let Some(last) = words.last() {
            let builtin = map_base_type(&[last.as_str()]).is_some() || is_c_type_keyword(last);
            if !builtin {
                name = words.pop().unwrap();
            }
        }
    }
    Ok((words, ptr_depth, name, is_array, false))
}

/// Map a full C declaration to an Alya type, given typedef aliases.
/// `for_param`: arrays decay to pointers (C semantics); `is_const_char_star`
/// selects `str` over `ptr` for `const char *`.
fn map_decl(
    decl: &str,
    aliases: &HashMap<String, String>,
    for_param: bool,
) -> Result<String, String> {
    let (words, ptr_depth, _name, is_array, _) = split_declarator(decl)?;
    if words.is_empty() {
        return Err("empty declaration".to_string());
    }
    // Synthetic function-pointer marker from split_declarator: a word.
    if words.len() == 1 && words[0] == "ptr" {
        return Ok("ptr".to_string());
    }
    // `struct Name` / `enum Name` / `union Name` references.
    if words[0] == "struct" {
        if words.len() != 2 {
            return Err("anonymous struct reference".to_string());
        }
        if ptr_depth > 0 || (for_param && is_array) {
            return Ok("ptr".to_string());
        }
        return Ok(words[1].clone());
    }
    if words[0] == "union" {
        return Err("union types have no Alya layout".to_string());
    }
    if words[0] == "enum" {
        // C enums are int-width; values travel as ints.
        return Ok("i32".to_string());
    }
    if ptr_depth > 0 {
        // `const char *` is the established `str` shape (package
        // practice); every other pointer is an opaque word. The match
        // is token-exact on the raw declarator (qualifiers are already
        // stripped from `words`): `unsigned char` merely contains the
        // substring "char" but is binary data, so it stays `ptr`; so
        // does `char *const p` (mutable pointee).
        if ptr_depth == 1 && !is_array {
            let toks: Vec<&str> = decl
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .filter(|s| !s.is_empty())
                .collect();
            let has_char = toks.contains(&"char");
            let has_const = toks.contains(&"const");
            let has_sign = toks.contains(&"unsigned") || toks.contains(&"signed");
            if has_char && has_const && !has_sign {
                return Ok("str".to_string());
            }
        }
        return Ok("ptr".to_string());
    }
    if for_param && is_array {
        return Ok("ptr".to_string());
    }
    if is_array {
        return Err("fixed-size array fields have no Alya layout".to_string());
    }
    let base = map_base_type(&words.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    if let Some(mapped) = base {
        if mapped == "void" {
            return Err("bare void value".to_string());
        }
        return Ok(mapped);
    }
    // A failed builtin combo (e.g. `long`, `char`) is unmappable, not
    // unknown: the words are C keywords, never a missing typedef.
    if words.iter().all(|w| is_c_type_keyword(w)) {
        return Err(format!("unmappable type '{}'", words.join(" ")));
    }
    // Transparent typedef alias?
    if words.len() == 1 {
        if let Some(aliased) = aliases.get(&words[0]) {
            return Ok(aliased.clone());
        }
        return Err(format!("unknown type '{}'", words[0]));
    }
    Err(format!("unmappable type '{}'", words.join(" ")))
}

/// Parse a `{...}` struct/enum body into comma-separated member decls.
fn split_members(body: &str) -> Vec<String> {
    let mut members = Vec::new();
    let mut current = String::new();
    let mut paren = 0i32;
    let mut brace = 0i32;
    let mut bracket = 0i32;
    for ch in body.chars() {
        match ch {
            '(' => paren += 1,
            ')' => paren -= 1,
            '{' => brace += 1,
            '}' => brace -= 1,
            '[' => bracket += 1,
            ']' => bracket -= 1,
            ',' if paren == 0 && brace == 0 && bracket == 0 => {
                members.push(current.trim().to_string());
                current = String::new();
                continue;
            }
            ';' if paren == 0 && brace == 0 && bracket == 0 => {
                members.push(current.trim().to_string());
                current = String::new();
                continue;
            }
            _ => {}
        }
        current.push(ch);
    }
    if !current.trim().is_empty() {
        members.push(current.trim().to_string());
    }
    members.into_iter().filter(|m| !m.is_empty()).collect()
}

/// Parse one top-level item, recording aliases along the way.
fn parse_item(item: &str, aliases: &mut HashMap<String, String>, items: &mut Vec<Item>) {
    let trimmed = item.trim();
    if trimmed.is_empty() {
        return;
    }
    // Preprocessor directives.
    if let Some(rest) = trimmed.strip_prefix('#') {
        let rest = rest.trim();
        if rest.starts_with("define") {
            let body = rest.strip_prefix("define").unwrap_or(rest).trim();
            // Function-like macros are out of scope.
            if let Some(paren) = body.find('(') {
                if body.find(char::is_whitespace).map_or(true, |ws| paren < ws) {
                    items.push(Item::Skipped {
                        reason: "function-like macro".to_string(),
                        source: trimmed.to_string(),
                    });
                    return;
                }
            }
            let mut parts = body.splitn(2, char::is_whitespace);
            let name = parts.next().unwrap_or("").to_string();
            let value = parts.next().unwrap_or("").trim().to_string();
            let is_ident = !name.is_empty()
                && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !name.chars().next().is_some_and(|c| c.is_numeric());
            let is_int = !value.is_empty()
                && (value.parse::<i128>().is_ok()
                    || value
                        .strip_prefix("0x")
                        .is_some_and(|h| i128::from_str_radix(h, 16).is_ok())
                    || value
                        .strip_prefix("0X")
                        .is_some_and(|h| i128::from_str_radix(h, 16).is_ok()));
            if is_ident && is_int {
                items.push(Item::Const { name, value });
            } else if is_ident && value.is_empty() {
                // Valueless flag macro. Include guards (`FOO_H`) are
                // structural: skip silently. Anything else is loud —
                // a flag with no value has no Alya mapping.
                let guard = name.ends_with("_H")
                    || name.ends_with("_HPP")
                    || name.ends_with("_H_")
                    || name.ends_with("_INCLUDED");
                if !guard {
                    items.push(Item::Skipped {
                        reason: "valueless flag macro".to_string(),
                        source: trimmed.to_string(),
                    });
                }
            } else {
                items.push(Item::Skipped {
                    reason: "non-integer macro".to_string(),
                    source: trimmed.to_string(),
                });
            }
            return;
        }
        // Structural directives carry no bindings.
        return;
    }
    // Lone closing brace (end of an `extern "C" { }` wrapper).
    if trimmed == "}" {
        return;
    }
    // `extern "C" { ... }` wrapper: recurse into the body.
    if trimmed.starts_with("extern") && trimmed.contains('{') {
        let start = trimmed.find('{').unwrap() + 1;
        let mut end = trimmed.rfind('}').unwrap_or(trimmed.len());
        if end < start {
            end = trimmed.len();
        }
        for inner in split_items(&trimmed[start..end]) {
            parse_item(&inner, aliases, items);
        }
        return;
    }
    // Single-decl `extern "C" proto;` (no braces).
    let body = if trimmed.starts_with("extern") {
        if let Some(semi) = trimmed.find(';') {
            trimmed[..semi].to_string()
        } else {
            items.push(Item::Skipped {
                reason: "malformed extern declaration".to_string(),
                source: trimmed.to_string(),
            });
            return;
        }
    } else {
        trimmed.to_string()
    };
    let body = body.trim().trim_end_matches(';').trim().to_string();
    // Strip a leading `extern "C"` / `extern` prefix on single decls.
    let body = if let Some(rest) = body.strip_prefix("extern") {
        let rest = rest.trim().trim_start_matches('"').to_string();
        if let Some(close) = rest.find('"') {
            rest[close + 1..].trim().to_string()
        } else {
            rest
        }
    } else {
        body
    };
    // Typedefs.
    if body.starts_with("typedef") {
        parse_typedef(&body, aliases, items, trimmed);
        return;
    }
    // `struct Name { ... };` definitions (forward decls end without braces).
    if body.starts_with("struct") {
        if !body.contains('{') {
            return; // Forward declaration: no layout, nothing to bind.
        }
        parse_struct_def(&body, None, aliases, items, trimmed);
        return;
    }
    if body.starts_with("union") {
        items.push(Item::Skipped {
            reason: "union types have no Alya layout".to_string(),
            source: trimmed.to_string(),
        });
        return;
    }
    if body.starts_with("enum") {
        parse_enum_def(&body, None, items, trimmed);
        return;
    }
    // Otherwise: function prototype (must contain parens).
    if !body.contains('(') {
        items.push(Item::Skipped {
            reason: "not a function prototype (globals cannot bind)".to_string(),
            source: trimmed.to_string(),
        });
        return;
    }
    parse_function(&body, items, trimmed, aliases);
}

fn parse_typedef(
    body: &str,
    aliases: &mut HashMap<String, String>,
    items: &mut Vec<Item>,
    source: &str,
) {
    let rest = body["typedef".len()..].trim();
    // `typedef struct ...` / `typedef enum ...` / `typedef union ...`.
    for keyword in ["struct", "union", "enum"] {
        if rest.starts_with(keyword) {
            if keyword == "union" {
                items.push(Item::Skipped {
                    reason: "union types have no Alya layout".to_string(),
                    source: source.to_string(),
                });
                return;
            }
            // Split trailing alias: `... } Alias` or `... } Alias[N]`.
            if let Some(close) = rest.rfind('}') {
                let tail = rest[close + 1..].trim().trim_end_matches(';').trim();
                let alias = tail
                    .split(|c: char| c.is_whitespace() || c == '[')
                    .next()
                    .unwrap_or("")
                    .to_string();
                if keyword == "struct" {
                    parse_struct_def(rest, Some(alias.clone()), aliases, items, source);
                    return;
                }
                parse_enum_def(rest, Some(alias.clone()), items, source);
                return;
            }
            // `typedef struct Name Alias;` (no body): transparent alias.
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() == 3 {
                let target = parts[1].to_string();
                let alias = parts[2].trim_end_matches(';').to_string();
                // Resolve struct/enum refs to their names; else record raw.
                aliases.insert(alias, target);
                return;
            }
            items.push(Item::Skipped {
                reason: "unsupported typedef shape".to_string(),
                source: source.to_string(),
            });
            return;
        }
    }
    // Function-pointer typedef: `typedef ret (*name)(args)` -> ptr alias.
    if rest.contains("(*") {
        if let Some(star) = rest.find("(*") {
            let after = &rest[star + 2..];
            if let Some(end) = after.find(')') {
                let name = after[..end].trim().trim_end_matches(';').to_string();
                if !name.is_empty() {
                    aliases.insert(name, "ptr".to_string());
                    return;
                }
            }
        }
        items.push(Item::Skipped {
            reason: "malformed function-pointer typedef".to_string(),
            source: source.to_string(),
        });
        return;
    }
    // Plain alias: `typedef <type> <name>;` (arrays decay: still the base).
    let clean = rest.trim_end_matches(';').trim();
    // Drop trailing array bounds on the alias (`typedef int arr_t[4]`).
    let (decl, _array) = match clean.find('[') {
        Some(idx) => (clean[..idx].trim(), true),
        None => (clean, false),
    };
    let mut parts: Vec<&str> = decl.split_whitespace().collect();
    if parts.is_empty() {
        items.push(Item::Skipped {
            reason: "empty typedef".to_string(),
            source: source.to_string(),
        });
        return;
    }
    // Pointers attach to the alias: `typedef char *cstr_t` / `typedef void *p`.
    let mut ptr_depth = 0usize;
    while parts.last().is_some_and(|p| *p == "*") {
        parts.pop();
        ptr_depth += 1;
    }
    // A `*` glued to the name (`*name`) — split it.
    let mut alias = parts.pop().unwrap_or("").to_string();
    while alias.starts_with('*') {
        alias.remove(0);
        ptr_depth += 1;
    }
    if alias.is_empty() {
        items.push(Item::Skipped {
            reason: "anonymous typedef".to_string(),
            source: source.to_string(),
        });
        return;
    }
    let fake_decl = if ptr_depth > 0 {
        format!("{} {}", parts.join(" "), "*".repeat(ptr_depth))
    } else {
        parts.join(" ")
    };
    match map_decl(&fake_decl, aliases, false) {
        Ok(mapped) => {
            aliases.insert(alias, mapped);
        }
        Err(reason) => items.push(Item::Skipped {
            reason: format!("typedef: {}", reason),
            source: source.to_string(),
        }),
    }
}

fn parse_struct_def(
    body: &str,
    alias: Option<String>,
    aliases: &mut HashMap<String, String>,
    items: &mut Vec<Item>,
    source: &str,
) {
    // Name: `struct Name {` or anonymous (`struct {`).
    let head_end = body.find('{').unwrap_or(body.len());
    let head = body[..head_end].trim();
    let struct_kw = head.strip_prefix("struct").unwrap_or(head).trim();
    // `typedef struct Name { ... } Alias` may carry both names.
    let struct_name = struct_kw
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    let name = if !struct_name.is_empty() {
        struct_name
    } else {
        alias.clone().unwrap_or_default()
    };
    if name.is_empty() {
        items.push(Item::Skipped {
            reason: "anonymous struct".to_string(),
            source: source.to_string(),
        });
        return;
    }
    let close = body.rfind('}').unwrap_or(body.len());
    let open = body.find('{').unwrap() + 1;
    let mut fields = Vec::new();
    for member in split_members(&body[open..close]) {
        let member = member.trim().trim_end_matches(';').trim().to_string();
        if member.is_empty() {
            continue;
        }
        // Nested struct/union member: no flat layout.
        if member.contains('{') {
            items.push(Item::Skipped {
                reason: format!("struct '{}': nested member has no flat layout", name),
                source: source.to_string(),
            });
            return;
        }
        // Bitfields have no Alya layout.
        if member.contains(':') && !member.contains("::") {
            // Careful: `::` never appears in C; any `:` here is a bitfield.
            items.push(Item::Skipped {
                reason: format!("struct '{}': bitfield member", name),
                source: source.to_string(),
            });
            return;
        }
        // Map first so width errors report precisely (an empty name
        // with brackets is an array field, not an anonymous member).
        let mapped = map_decl(&member, aliases, false);
        let (_words, _ptr, fname, _arr, _) = match split_declarator(&member) {
            Ok(parsed) => parsed,
            Err(reason) => {
                items.push(Item::Skipped {
                    reason: format!("struct '{}': {}", name, reason),
                    source: source.to_string(),
                });
                return;
            }
        };
        let mapped = match mapped {
            Ok(mapped) => mapped,
            Err(reason) => {
                items.push(Item::Skipped {
                    reason: format!("struct '{}': {}", name, reason),
                    source: source.to_string(),
                });
                return;
            }
        };
        if fname.is_empty() {
            items.push(Item::Skipped {
                reason: format!("struct '{}': anonymous member", name),
                source: source.to_string(),
            });
            return;
        }
        fields.push((fname, mapped));
    }
    // Register the struct name for later references.
    aliases.insert(name.clone(), name.clone());
    if let Some(a) = alias {
        if a != name {
            aliases.insert(a, name.clone());
        }
    }
    items.push(Item::Struct { name, fields });
}

fn parse_enum_def(body: &str, alias: Option<String>, items: &mut Vec<Item>, source: &str) {
    let head_end = body.find('{').unwrap_or(body.len());
    let head = body[..head_end].trim();
    let enum_kw = head.strip_prefix("enum").unwrap_or(head).trim();
    let enum_name = enum_kw.split_whitespace().next().unwrap_or("").to_string();
    let name = if !enum_name.is_empty() {
        Some(enum_name)
    } else {
        alias
    };
    let (Some(open), Some(close)) = (body.find('{'), body.rfind('}')) else {
        // Bare `enum Name;`: nothing to bind.
        return;
    };
    let mut variants = Vec::new();
    let mut next: i128 = 0;
    for member in split_members(&body[open + 1..close]) {
        let member = member.trim().trim_end_matches(';').trim().to_string();
        if member.is_empty() {
            continue;
        }
        // `Name = value` or plain `Name`.
        let (vname, value) = match member.find('=') {
            Some(idx) => {
                let vname = member[..idx].trim().to_string();
                let vtext = member[idx + 1..].trim();
                // Integer literal (decimal/hex, optional sign/suffix).
                let clean = vtext.trim_end_matches(['u', 'U', 'l', 'L']).trim();
                let parsed = if let Some(hex) = clean
                    .strip_prefix("0x")
                    .or_else(|| clean.strip_prefix("0X"))
                {
                    i128::from_str_radix(hex, 16).ok()
                } else if let Some(oct) = clean.strip_prefix('0') {
                    if oct.is_empty() {
                        Some(0)
                    } else {
                        i128::from_str_radix(oct, 8).ok()
                    }
                } else {
                    clean.parse::<i128>().ok()
                };
                match parsed {
                    Some(v) => (vname, v),
                    None => {
                        items.push(Item::Skipped {
                            reason: "non-integer enumerator".to_string(),
                            source: source.to_string(),
                        });
                        return;
                    }
                }
            }
            None => (member, next),
        };
        if vname.is_empty()
            || !vname.chars().all(|c| c.is_alphanumeric() || c == '_')
            || vname.chars().next().is_some_and(|c| c.is_numeric())
        {
            items.push(Item::Skipped {
                reason: "invalid enumerator name".to_string(),
                source: source.to_string(),
            });
            return;
        }
        next = value + 1;
        variants.push((vname, value));
    }
    items.push(Item::Enum { name, variants });
}

fn parse_function(
    body: &str,
    items: &mut Vec<Item>,
    source: &str,
    aliases: &HashMap<String, String>,
) {
    // Split `ret name(params)`: params are the last balanced parens.
    let open = match body.find('(') {
        Some(idx) => idx,
        None => {
            items.push(Item::Skipped {
                reason: "not a function prototype".to_string(),
                source: source.to_string(),
            });
            return;
        }
    };
    let close = match body.rfind(')') {
        Some(idx) => idx,
        None => {
            items.push(Item::Skipped {
                reason: "unbalanced prototype".to_string(),
                source: source.to_string(),
            });
            return;
        }
    };
    let head = body[..open].trim();
    let argtext = body[open + 1..close].trim();
    // Head: `<ret> <name>` — strip storage qualifiers.
    let mut head_words: Vec<&str> = head.split_whitespace().collect();
    head_words.retain(|w| !matches!(*w, "extern" | "static" | "inline" | "_Noreturn"));
    // Leading API-annotation macros (`SQLITE_API int f(...)`) are
    // handled at the return-type mapping below (typedef-first retry),
    // so this only drops the unambiguous 3+-word case up front.
    // A real 2-word `TYPE name` head never strips.
    if head_words.len() > 2
        && head_words[0]
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
        && !aliases.contains_key(head_words[0])
    {
        head_words.remove(0);
    }
    if head_words.is_empty() {
        items.push(Item::Skipped {
            reason: "missing return type".to_string(),
            source: source.to_string(),
        });
        return;
    }
    let mut fname = head_words.pop().unwrap().to_string();
    while fname.starts_with('*') {
        // `ret *name(...)`: function returning a pointer.
        fname.remove(0);
    }
    if fname.is_empty()
        || !fname.chars().all(|c| c.is_alphanumeric() || c == '_')
        || fname.chars().next().is_some_and(|c| c.is_numeric())
    {
        items.push(Item::Skipped {
            reason: "invalid function name".to_string(),
            source: source.to_string(),
        });
        return;
    }
    // Pointer returns ride the declarator star. Rebuild from the
    // raw head (qualifiers intact: `const char *greet` must keep its
    // `const` for the str rule); the name token carries the star.
    let head_toks: Vec<&str> = body[..open].split_whitespace().collect();
    let name_tok = head_toks.last().unwrap_or(&"").to_string();
    let mut ret_full = head_toks[..head_toks.len().saturating_sub(1)].join(" ");
    if name_tok.starts_with('*') {
        ret_full.push_str(" *");
    }
    // Typedef-first retry: map as-is (a real `DWORD` alias wins);
    // on failure, drop ALL-CAPS non-alias words anywhere in the head
    // (`SQLITE_API`, mid-head `SQLITE_CDECL`) and map again. Whatever
    // still fails skips loudly below.
    let mut ret = map_decl(&ret_full, aliases, false);
    if ret.is_err() {
        let filtered: Vec<&str> = ret_full
            .split_whitespace()
            .filter(|w| {
                let bare = w.trim_matches('*');
                !(bare
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
                    && !aliases.contains_key(bare))
            })
            .collect();
        if filtered.len() != ret_full.split_whitespace().count() {
            ret = map_decl(&filtered.join(" "), aliases, false);
        }
    }
    let ret = if ret_full == "void" {
        None
    } else {
        match ret {
            Ok(mapped) => {
                if mapped == "void" {
                    None
                } else {
                    Some(mapped)
                }
            }
            Err(reason) => {
                items.push(Item::Skipped {
                    reason: format!("function '{}': return: {}", fname, reason),
                    source: source.to_string(),
                });
                return;
            }
        }
    };
    // Params.
    let mut params = Vec::new();
    if !argtext.is_empty() && argtext != "void" {
        // Top-level comma split (function-pointer params hold their commas).
        let mut depth = 0i32;
        let mut current = String::new();
        let mut parts = Vec::new();
        for ch in argtext.chars() {
            match ch {
                '(' => {
                    depth += 1;
                    current.push(ch);
                }
                ')' => {
                    depth -= 1;
                    current.push(ch);
                }
                ',' if depth == 0 => {
                    parts.push(current.trim().to_string());
                    current = String::new();
                }
                _ => current.push(ch),
            }
        }
        if !current.trim().is_empty() {
            parts.push(current.trim().to_string());
        }
        let mut counter = 0usize;
        for part in parts {
            let part = part.trim();
            if part == "..." {
                items.push(Item::Skipped {
                    reason: format!("function '{}': variadic", fname),
                    source: source.to_string(),
                });
                return;
            }
            // `void` lone param is handled above; a stray `void` among
            // params is malformed.
            let (_words, _ptr, mut pname, _arr, _) = match split_declarator(part) {
                Ok(parsed) => parsed,
                Err(reason) => {
                    items.push(Item::Skipped {
                        reason: format!("function '{}': param: {}", fname, reason),
                        source: source.to_string(),
                    });
                    return;
                }
            };
            if pname.is_empty() {
                pname = format!("arg{}", counter);
                counter += 1;
            }
            match map_decl(part, aliases, true) {
                Ok(mapped) => params.push((pname, mapped)),
                Err(reason) => {
                    items.push(Item::Skipped {
                        reason: format!("function '{}': param '{}': {}", fname, pname, reason),
                        source: source.to_string(),
                    });
                    return;
                }
            }
        }
    }
    items.push(Item::Function {
        name: fname,
        params,
        ret,
    });
}

/// Generate Alya source from a C header.
pub fn generate(header_src: &str, opts: &BindgenOptions) -> String {
    let cleaned = strip_comments(header_src);
    let mut aliases: HashMap<String, String> = HashMap::new();
    let mut items: Vec<Item> = Vec::new();
    for item in split_items(&cleaned) {
        parse_item(&item, &mut aliases, &mut items);
    }
    // Platform `#if` branches both survive filtering, so the same
    // function can arrive twice: first wins, the rest are loud skips
    // (C has no overloading; a duplicate is a conflict, not a choice).
    let mut seen_functions: HashMap<String, String> = HashMap::new();
    let mut deduped: Vec<Item> = Vec::new();
    for item in items {
        match &item {
            Item::Function { name, .. } => {
                if let Some(first) = seen_functions.get(name) {
                    deduped.push(Item::Skipped {
                        reason: format!("duplicate of '{}'", first),
                        source: format!("function {}", name),
                    });
                    continue;
                }
                seen_functions.insert(name.clone(), name.clone());
                deduped.push(item);
            }
            _ => deduped.push(item),
        }
    }
    let items = deduped;
    let mut out = String::new();
    out.push_str("## Generated by `alya bindgen`; do not hand-edit, regenerate.\n");
    out.push_str("## Unmappable declarations become `-- SKIPPED:` comments, never guesses.\n\n");
    for item in &items {
        match item {
            Item::Struct { name, fields } => {
                out.push_str("@repr(C)\n");
                out.push_str(&format!("struct {}\n", name));
                for (i, (fname, ftype)) in fields.iter().enumerate() {
                    if i + 1 == fields.len() {
                        out.push_str(&format!("    {}: {}\n", fname, ftype));
                    } else {
                        out.push_str(&format!("    {}: {},\n", fname, ftype));
                    }
                }
                out.push_str("end\n\n");
            }
            Item::Enum { name, variants } => {
                if let Some(ename) = name {
                    out.push_str(&format!("enum {}\n", ename));
                    for (vname, value) in variants {
                        out.push_str(&format!("    {} = {}\n", vname, value));
                    }
                    out.push_str("end\n\n");
                } else {
                    for (vname, value) in variants {
                        out.push_str(&format!("const {} = {}\n", vname, value));
                    }
                    out.push('\n');
                }
            }
            Item::Const { name, value } => {
                out.push_str(&format!("const {} = {}\n\n", name, value));
            }
            // Functions share one extern block below; skips trail last.
            Item::Function { .. } | Item::Skipped { .. } => {}
        }
    }
    // One shared extern block for all functions.
    let functions: Vec<&Item> = items
        .iter()
        .filter(|item| matches!(item, Item::Function { .. }))
        .collect();
    if !functions.is_empty() {
        match &opts.from_lib {
            Some(lib) => out.push_str(&format!("extern \"C\" from \"{}\"\n", lib)),
            None => out.push_str("extern \"C\"\n"),
        }
        for item in functions {
            if let Item::Function { name, params, ret } = item {
                let param_list = params
                    .iter()
                    .map(|(pname, ptype)| format!("{}: {}", pname, ptype))
                    .collect::<Vec<_>>()
                    .join(", ");
                match ret {
                    Some(rtype) => out.push_str(&format!(
                        "    function {}({}) -> {}\n",
                        name, param_list, rtype
                    )),
                    None => out.push_str(&format!("    function {}({})\n", name, param_list)),
                }
            }
        }
        out.push_str("end\n\n");
    }
    // Skipped items trail the file as honest comments.
    let skipped: Vec<&Item> = items
        .iter()
        .filter(|item| matches!(item, Item::Skipped { .. }))
        .collect();
    if !skipped.is_empty() {
        out.push_str("# bindgen skipped the following declarations:\n");
        for item in skipped {
            if let Item::Skipped { reason, source } = item {
                let one_line = source.split_whitespace().collect::<Vec<_>>().join(" ");
                let short = one_line.chars().take(100).collect::<String>();
                out.push_str(&format!("# -- SKIPPED ({}): {}\n", reason, short));
            }
        }
    }
    out
}

/// CLI entry: read a header, write bindings to a file or stdout.
pub fn run_bindgen(
    header_path: &str,
    from_lib: Option<&str>,
    out_path: Option<&str>,
) -> Result<(), String> {
    let src = std::fs::read_to_string(header_path)
        .map_err(|e| format!("Error: cannot read header '{}': {}", header_path, e))?;
    let opts = BindgenOptions {
        from_lib: from_lib.map(|s| s.to_string()),
    };
    let generated = generate(&src, &opts);
    match out_path {
        Some(path) => std::fs::write(path, &generated)
            .map_err(|e| format!("Error: cannot write '{}': {}", path, e)),
        None => {
            print!("{}", generated);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen(header: &str) -> String {
        generate(
            header,
            &BindgenOptions {
                from_lib: Some("demo".to_string()),
            },
        )
    }

    #[test]
    fn maps_function_shapes() {
        let out = gen("int add(int a, int b);\n\
             void reset(void);\n\
             const char *greet(const char *name, int loud);\n\
             unsigned char *fetch(int slot, char *buf, long long *out);\n\
             void *alloc_blob(size_t n);\n\
             uint32_t crc32(uint32_t crc, const uint8_t *buf, size_t len);\n\
             int on_event(int code, void (*cb)(int));\n\
             double ratio(double x, int y);\n");
        for line in [
            "function add(a: i32, b: i32) -> i32",
            "function reset()",
            "function greet(name: str, loud: i32) -> str",
            "function fetch(slot: i32, buf: ptr, out: ptr) -> ptr",
            "function alloc_blob(n: usize) -> ptr",
            "function crc32(crc: u32, buf: ptr, len: usize) -> u32",
            "function on_event(code: i32, cb: ptr) -> i32",
            "function ratio(x: f64, y: i32) -> f64",
            "extern \"C\" from \"demo\"",
        ] {
            assert!(out.contains(line), "missing '{}'\n{}", line, out);
        }
    }

    #[test]
    fn maps_structs_enums_typedefs_defines() {
        let out = gen(
            "typedef struct {\n    int fd;\n    long long offset;\n    uint8_t flags;\n    void *ctx;\n} handle_t;\n\
             struct point { int x; int y; };\n\
             typedef enum { OK = 0, RETRY = 2, FAIL } status_t;\n\
             enum { ANON_A = 5, ANON_B };\n\
             typedef unsigned int uword_t;\n\
             typedef void (*cb_t)(int code);\n\
             #define MAX_SLOTS 64\n\
             #define MAGIC 0x5A11\n\
             uword_t slots_used(handle_t *h, status_t *st);\n",
        );
        for line in [
            "@repr(C)",
            "struct handle_t",
            "fd: i32,",
            "offset: i64,",
            "flags: u8,",
            "ctx: ptr",
            "struct point",
            "x: i32,",
            "enum status_t",
            "OK = 0",
            "RETRY = 2",
            "FAIL = 3",
            "const ANON_A = 5",
            "const ANON_B = 6",
            "const MAX_SLOTS = 64",
            "const MAGIC = 0x5A11",
            // Typedefs resolve (Alya extern blocks have no aliases).
            "function slots_used(h: ptr, st: ptr) -> u32",
        ] {
            assert!(out.contains(line), "missing '{}'\n{}", line, out);
        }
    }

    #[test]
    fn skips_loudly_never_guesses() {
        let out = gen("long platform_word(void);\n\
             char classify(int c);\n\
             int printf_like(const char *fmt, ...);\n\
             union word { int i; float f; };\n\
             struct with_bit { unsigned int flag : 1; };\n\
             struct with_array { char name[32]; };\n\
             #define FN(x) ((x) + 1)\n\
             #define GREETING \"hi\"\n\
             #define FEATURE_X\n\
             extern int shared_counter;\n\
             int ok_after_skips(void);\n");
        for marker in [
            "SKIPPED (function 'platform_word': return: unmappable type 'long')",
            "SKIPPED (function 'classify': return: unmappable type 'char')",
            "SKIPPED (function 'printf_like': variadic)",
            "SKIPPED (union types have no Alya layout)",
            "SKIPPED (struct 'with_bit': bitfield member)",
            "SKIPPED (struct 'with_array': fixed-size array fields have no Alya layout)",
            "SKIPPED (function-like macro)",
            "SKIPPED (non-integer macro)",
            "SKIPPED (valueless flag macro)",
            "SKIPPED (not a function prototype (globals cannot bind))",
            "function ok_after_skips()",
        ] {
            assert!(out.contains(marker), "missing '{}'\n{}", marker, out);
        }
    }

    #[test]
    fn survives_real_world_wrappers() {
        // __cplusplus guards, include lines, block comments, and an
        // `extern "C" { }` wrapper must not leak into the output.
        let out = gen("#ifndef DEMO_H\n#define DEMO_H\n#include <stddef.h>\n\
             /* multi\n    line comment */\n\
             #ifdef __cplusplus\nextern \"C\" {\n#endif\n\
             // line comment\n\
             size_t measure(const char *s);\n\
             #ifdef __cplusplus\n}\n#endif\n#endif\n");
        assert!(out.contains("function measure(s: str) -> usize"), "{}", out);
        // (The header comment mentions `-- SKIPPED:`; assert on markers.)
        assert!(!out.contains("# -- SKIPPED"), "{}", out);
        assert!(!out.contains("__cplusplus"), "{}", out);
    }

    #[test]
    fn unnamed_params_get_positions() {
        let out = gen("int compare(const void *, const void *);\n");
        assert!(
            out.contains("function compare(arg0: ptr, arg1: ptr) -> i32"),
            "{}",
            out
        );
    }

    #[test]
    fn strips_api_annotation_macros() {
        // `SQLITE_API`-style leading macros are not types; a real
        // 2-word typedef head never strips.
        let out =
            gen("#define SQLITE_API\nint SQLITE_CDECL sqlite3_open(const char *name, int *db);\n");
        assert!(
            out.contains("function sqlite3_open(name: str, db: ptr) -> i32"),
            "{}",
            out
        );
    }

    #[test]
    fn char_pointer_flavors() {
        // `const char *` is the str shape; unsigned bytes and
        // const-qualified pointers stay opaque words.
        let out = gen("const char *who(void);\n\
             unsigned char *raw(int slot);\n\
             char *mutable_buf(char *dst);\n");
        for line in [
            "function who() -> str",
            "function raw(slot: i32) -> ptr",
            "function mutable_buf(dst: ptr) -> ptr",
        ] {
            assert!(out.contains(line), "missing '{}'\n{}", line, out);
        }
    }
}
