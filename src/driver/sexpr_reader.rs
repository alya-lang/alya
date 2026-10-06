//! S-expression → AST reader (hybrid B first slice, `spike/selfhost/`).
//!
//! Pure inverse of `ast_sexpr.rs`: canonical dump text → `ast::Program`.
//! No format change, no AST change (spans are absent end-to-end — see
//! `CODEGEN_FEASIBILITY.md` bridge design).
//!
//! Rules mirrored from the dumper:
//! - Strings are JSON-escaped (`qs`/`esc_char`), incl. `\uXXXX` and
//!   surrogate pairs. NUL never reaches the reader (dumper truncates).
//! - Floats parse decimal/`inf`/`NaN` text to f64; shortest-roundtrip
//!   (`{:?}`) guarantees re-dump identity for Rust-produced dumps.
//! - Integers are decimal `i128` (AST type). Overflow is an error.
//! - `_` → None, `true`/`false` → bool. Folded desugars pass through
//!   untouched — the reader never re-desugars.
//! - `0|0|error|...` lines never reach the reader (driver checks the
//!   frontend exit code first); feeding one here is an error.

use crate::ast::{Attribute, ExternFnDecl, ExternParam};
use crate::ast::{BinaryOp, Expr, ImportSymbol, InterfaceMethod, Program, Stmt, UnaryOp};

/// Parsed S-expression node.
#[derive(Debug, Clone, PartialEq)]
enum Node {
    List(Vec<Node>),
    /// JSON-decoded quoted string.
    Str(String),
    /// Bare atom (`num`, `true`, `_`, `+`, `1.0`, ...).
    Atom(String),
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

/// JSON-unescape the inside of a `qs` atom (surrounding quotes removed).
/// Accepts the full JSON set (`\" \\ \/ \b \f \n \r \t \uXXXX` with
/// surrogate pairs); the dumper only emits a subset.
fn json_unescape(raw: &str) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    let mut it = raw.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let e = it.next().ok_or("truncated escape")?;
        match e {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            '/' => out.push('/'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'u' => {
                let hex: String = (0..4)
                    .map(|_| it.next().ok_or("truncated \\u escape"))
                    .collect::<Result<String, _>>()?;
                let hi = u32::from_str_radix(&hex, 16).map_err(|_| format!("bad \\u{hex}"))?;
                // High surrogate: must be followed by a low surrogate.
                if (0xD800..0xDC00).contains(&hi) {
                    if it.next() != Some('\\') || it.next() != Some('u') {
                        return Err("lone high surrogate".to_string());
                    }
                    let hex2: String = (0..4)
                        .map(|_| it.next().ok_or("truncated low surrogate"))
                        .collect::<Result<String, _>>()?;
                    let lo =
                        u32::from_str_radix(&hex2, 16).map_err(|_| format!("bad \\u{hex2}"))?;
                    if !(0xDC00..0xE000).contains(&lo) {
                        return Err("bad low surrogate".to_string());
                    }
                    let cp = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                    out.push(char::from_u32(cp).ok_or("bad surrogate pair")?);
                } else if (0xDC00..0xE000).contains(&hi) {
                    return Err("lone low surrogate".to_string());
                } else {
                    out.push(char::from_u32(hi).ok_or(format!("bad \\u{hex}"))?);
                }
            }
            _ => return Err(format!("bad escape \\{e}")),
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    LParen,
    RParen,
    Str(String),
    Atom(String),
}

fn tokenize(text: &str) -> Result<Vec<Tok>, String> {
    let b = text.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if is_ws(b[i]) {
            i += 1;
            continue;
        }
        match b[i] {
            b'(' => {
                toks.push(Tok::LParen);
                i += 1;
            }
            b')' => {
                toks.push(Tok::RParen);
                i += 1;
            }
            b'"' => {
                i += 1; // open quote
                let mut raw = String::new();
                let mut closed = false;
                while i < b.len() {
                    let c = b[i] as char;
                    if c == '"' {
                        closed = true;
                        i += 1;
                        break;
                    }
                    if c == '\\' {
                        if i + 1 >= b.len() {
                            return Err("unterminated string escape".to_string());
                        }
                        raw.push('\\');
                        raw.push(b[i + 1] as char);
                        i += 2;
                        // `\u` body (4 hex) rides along raw; validated later.
                        continue;
                    }
                    // Raw UTF-8: consume the full char, not one byte.
                    let s = &text[i..];
                    let ch = s.chars().next().ok_or("bad utf8 in string")?;
                    // A literal newline inside a string is rejected: the
                    // dumper always escapes control chars.
                    if ch == '\n' || ch == '\r' {
                        return Err("literal newline in string".to_string());
                    }
                    raw.push(ch);
                    i += ch.len_utf8();
                }
                if !closed {
                    return Err("unterminated string".to_string());
                }
                toks.push(Tok::Str(json_unescape(&raw)?));
            }
            _ => {
                let start = i;
                while i < b.len() && !is_ws(b[i]) && b[i] != b'(' && b[i] != b')' && b[i] != b'"' {
                    i += 1;
                }
                let atom = text[start..i].to_string();
                if atom.is_empty() {
                    return Err("empty atom".to_string());
                }
                toks.push(Tok::Atom(atom));
            }
        }
    }
    Ok(toks)
}

fn parse_node(toks: &[Tok], pos: &mut usize) -> Result<Node, String> {
    if *pos >= toks.len() {
        return Err("unexpected EOF".to_string());
    }
    match &toks[*pos] {
        Tok::LParen => {
            *pos += 1;
            let mut items = Vec::new();
            loop {
                if *pos >= toks.len() {
                    return Err("unterminated list".to_string());
                }
                if toks[*pos] == Tok::RParen {
                    *pos += 1;
                    break;
                }
                items.push(parse_node(toks, pos)?);
            }
            Ok(Node::List(items))
        }
        Tok::RParen => Err("unexpected ')'".to_string()),
        Tok::Str(s) => {
            *pos += 1;
            Ok(Node::Str(s.clone()))
        }
        Tok::Atom(a) => {
            *pos += 1;
            Ok(Node::Atom(a.clone()))
        }
    }
}

// --- Small accessors ---

fn as_list(n: &Node) -> Result<&[Node], String> {
    match n {
        Node::List(v) => Ok(v),
        _ => Err("expected list".to_string()),
    }
}

fn head_is(items: &[Node], h: &str) -> bool {
    matches!(items.first(), Some(Node::Atom(a)) if a == h)
}

fn items_head(items: &[Node]) -> &str {
    match items.first() {
        Some(Node::Atom(a)) => a.as_str(),
        _ => "?",
    }
}

fn expect_arity(items: &[Node], n: usize) -> Result<(), String> {
    if items.len() - 1 == n {
        Ok(())
    } else {
        Err(format!(
            "({}) wants {} args, got {}",
            items_head(items),
            n,
            items.len() - 1
        ))
    }
}

fn as_atom_str(n: &Node) -> Result<&str, String> {
    match n {
        Node::Atom(a) => Ok(a.as_str()),
        _ => Err("expected bare atom".to_string()),
    }
}

fn as_quoted(n: &Node) -> Result<&str, String> {
    match n {
        Node::Str(s) => Ok(s.as_str()),
        _ => Err("expected quoted string".to_string()),
    }
}

fn parse_bool(n: &Node) -> Result<bool, String> {
    match n {
        Node::Atom(a) if a == "true" => Ok(true),
        Node::Atom(a) if a == "false" => Ok(false),
        _ => Err("expected true/false".to_string()),
    }
}

fn parse_opt_string(n: &Node) -> Result<Option<String>, String> {
    match n {
        Node::Atom(a) if a == "_" => Ok(None),
        Node::Str(s) => Ok(Some(s.clone())),
        _ => Err("expected quoted string or _".to_string()),
    }
}

fn parse_opt_expr(n: &Node) -> Result<Option<Expr>, String> {
    match n {
        Node::Atom(a) if a == "_" => Ok(None),
        _ => Ok(Some(parse_expr(n)?)),
    }
}

// --- Expr ---

fn binop_from(atom: &str) -> Result<BinaryOp, String> {
    Ok(match atom {
        "+" => BinaryOp::Add,
        "-" => BinaryOp::Subtract,
        "*" => BinaryOp::Multiply,
        "/" => BinaryOp::Divide,
        "%" => BinaryOp::Modulo,
        "==" => BinaryOp::Equal,
        "!=" => BinaryOp::NotEqual,
        "<" => BinaryOp::Less,
        ">" => BinaryOp::Greater,
        "<=" => BinaryOp::LessEqual,
        ">=" => BinaryOp::GreaterEqual,
        "and" => BinaryOp::And,
        "or" => BinaryOp::Or,
        "&" => BinaryOp::BitAnd,
        "|" => BinaryOp::BitOr,
        "^" => BinaryOp::BitXor,
        "<<" => BinaryOp::Shl,
        ">>" => BinaryOp::Shr,
        "in" => BinaryOp::In,
        "not-in" => BinaryOp::NotIn,
        ".." => BinaryOp::Range,
        "..=" => BinaryOp::RangeInclusive,
        _ => return Err(format!("unknown binop {atom:?}")),
    })
}

fn unop_from(atom: &str) -> Result<UnaryOp, String> {
    Ok(match atom {
        "-" => UnaryOp::Negate,
        "not" => UnaryOp::Not,
        "~" => UnaryOp::BitNot,
        _ => return Err(format!("unknown unop {atom:?}")),
    })
}

/// `(a b ...)` with no head tag (call args, array elems): every element
/// must itself be an Expr.
fn parse_expr_seq(n: &Node) -> Result<Vec<Expr>, String> {
    as_list(n)?.iter().map(parse_expr).collect()
}

fn parse_expr(n: &Node) -> Result<Expr, String> {
    let items = as_list(n)?;
    if items.is_empty() || !matches!(items[0], Node::Atom(_)) {
        return Err("expr needs a head atom".to_string());
    }
    let h = items_head(items);
    match h {
        "num" => {
            expect_arity(items, 1)?;
            let a = as_atom_str(&items[1])?;
            a.parse::<i128>()
                .map(Expr::Number)
                .map_err(|_| format!("bad int {a:?}"))
        }
        "float" => {
            expect_arity(items, 1)?;
            let a = as_atom_str(&items[1])?;
            a.parse::<f64>()
                .map(Expr::Float)
                .map_err(|_| format!("bad float {a:?}"))
        }
        "str" => {
            expect_arity(items, 1)?;
            Ok(Expr::String(as_quoted(&items[1])?.to_string()))
        }
        "ident" => {
            expect_arity(items, 1)?;
            Ok(Expr::Identifier(as_quoted(&items[1])?.to_string()))
        }
        "null" => {
            expect_arity(items, 0)?;
            Ok(Expr::Null)
        }
        "binop" => {
            expect_arity(items, 3)?;
            Ok(Expr::Binary {
                op: binop_from(as_atom_str(&items[1])?)?,
                left: Box::new(parse_expr(&items[2])?),
                right: Box::new(parse_expr(&items[3])?),
            })
        }
        "unop" => {
            expect_arity(items, 2)?;
            Ok(Expr::Unary {
                op: unop_from(as_atom_str(&items[1])?)?,
                expr: Box::new(parse_expr(&items[2])?),
            })
        }
        "call" => {
            expect_arity(items, 2)?;
            Ok(Expr::Call {
                name: as_quoted(&items[1])?.to_string(),
                args: parse_expr_seq(&items[2])?,
            })
        }
        "interp" => {
            expect_arity(items, 1)?;
            Ok(Expr::InterpolatedString(parse_expr_seq(&items[1])?))
        }
        "array" => {
            expect_arity(items, 1)?;
            Ok(Expr::Array(parse_expr_seq(&items[1])?))
        }
        "index" => {
            expect_arity(items, 2)?;
            Ok(Expr::Index {
                array: Box::new(parse_expr(&items[1])?),
                index: Box::new(parse_expr(&items[2])?),
            })
        }
        "field" => {
            expect_arity(items, 2)?;
            Ok(Expr::FieldAccess {
                object: Box::new(parse_expr(&items[1])?),
                field: as_quoted(&items[2])?.to_string(),
            })
        }
        "struct-init" => {
            expect_arity(items, 2)?;
            let mut fields = Vec::new();
            for f in as_list(&items[2])? {
                let pair = as_list(f)?;
                if pair.len() != 2 {
                    return Err("struct-init field wants (name expr)".to_string());
                }
                fields.push((as_quoted(&pair[0])?.to_string(), parse_expr(&pair[1])?));
            }
            Ok(Expr::StructInit {
                name: as_quoted(&items[1])?.to_string(),
                fields,
            })
        }
        "map" => {
            expect_arity(items, 1)?;
            let mut kvs = Vec::new();
            for kv in as_list(&items[1])? {
                let pair = as_list(kv)?;
                if pair.len() != 2 {
                    return Err("map entry wants (k v)".to_string());
                }
                kvs.push((parse_expr(&pair[0])?, parse_expr(&pair[1])?));
            }
            Ok(Expr::Map(kvs))
        }
        "ternary" => {
            expect_arity(items, 3)?;
            Ok(Expr::Ternary {
                condition: Box::new(parse_expr(&items[1])?),
                then_branch: Box::new(parse_expr(&items[2])?),
                else_branch: Box::new(parse_expr(&items[3])?),
            })
        }
        "coalesce" => {
            expect_arity(items, 2)?;
            Ok(Expr::NullCoalesce {
                value: Box::new(parse_expr(&items[1])?),
                default: Box::new(parse_expr(&items[2])?),
            })
        }
        "opt-field" => {
            expect_arity(items, 2)?;
            Ok(Expr::OptionalFieldAccess {
                object: Box::new(parse_expr(&items[1])?),
                field: as_quoted(&items[2])?.to_string(),
            })
        }
        "opt-index" => {
            expect_arity(items, 2)?;
            Ok(Expr::OptionalIndex {
                array: Box::new(parse_expr(&items[1])?),
                index: Box::new(parse_expr(&items[2])?),
            })
        }
        "opt-call" => {
            expect_arity(items, 2)?;
            Ok(Expr::OptionalCall {
                callee: as_quoted(&items[1])?.to_string(),
                args: parse_expr_seq(&items[2])?,
            })
        }
        "unwrap" => {
            expect_arity(items, 1)?;
            Ok(Expr::ForceUnwrap(Box::new(parse_expr(&items[1])?)))
        }
        "is" => {
            expect_arity(items, 3)?;
            Ok(Expr::TypeCheck {
                expr: Box::new(parse_expr(&items[1])?),
                target: as_quoted(&items[2])?.to_string(),
                negated: parse_bool(&items[3])?,
            })
        }
        "as" => {
            expect_arity(items, 2)?;
            Ok(Expr::Cast {
                expr: Box::new(parse_expr(&items[1])?),
                target: as_quoted(&items[2])?.to_string(),
            })
        }
        _ => Err(format!("unknown expr head {h:?}")),
    }
}

// --- Blocks / attrs ---

fn parse_block(n: &Node) -> Result<Vec<Stmt>, String> {
    let items = as_list(n)?;
    if !head_is(items, "block") {
        return Err("expected (block ...)".to_string());
    }
    items[1..].iter().map(parse_stmt).collect()
}

fn parse_opt_block(n: &Node) -> Result<Option<Vec<Stmt>>, String> {
    match n {
        Node::Atom(a) if a == "_" => Ok(None),
        _ => Ok(Some(parse_block(n)?)),
    }
}

fn parse_attrs(n: &Node) -> Result<Vec<Attribute>, String> {
    let mut out = Vec::new();
    for a in as_list(n)? {
        let pair = as_list(a)?;
        if pair.len() != 2 {
            return Err("attr wants (name (args))".to_string());
        }
        let mut args = Vec::new();
        for arg in as_list(&pair[1])? {
            let kv = as_list(arg)?;
            if kv.len() != 2 {
                return Err("attr arg wants (key value)".to_string());
            }
            let key = match &kv[0] {
                Node::Atom(a) if a == "_" => None,
                Node::Str(s) => Some(s.clone()),
                _ => return Err("attr key wants string or _".to_string()),
            };
            args.push((key, as_quoted(&kv[1])?.to_string()));
        }
        out.push(Attribute {
            name: as_quoted(&pair[0])?.to_string(),
            args,
        });
    }
    Ok(out)
}

fn parse_quoted_seq(n: &Node) -> Result<Vec<String>, String> {
    as_list(n)?
        .iter()
        .map(|e| Ok(as_quoted(e)?.to_string()))
        .collect()
}

fn parse_opt_str_seq(n: &Node) -> Result<Vec<Option<String>>, String> {
    as_list(n)?.iter().map(parse_opt_string).collect()
}

fn parse_opt_expr_seq(n: &Node) -> Result<Vec<Option<Expr>>, String> {
    as_list(n)?.iter().map(parse_opt_expr).collect()
}

// --- Stmt ---

fn parse_stmt(n: &Node) -> Result<Stmt, String> {
    let items = as_list(n)?;
    if items.is_empty() || !matches!(items[0], Node::Atom(_)) {
        return Err("stmt needs a head atom".to_string());
    }
    let h = items_head(items);
    match h {
        "import" => {
            expect_arity(items, 3)?;
            // `None` symbols dump as `(_)`; `Some([])` as `()`.
            let syms_node = as_list(&items[3])?;
            let symbols =
                if syms_node.len() == 1 && matches!(&syms_node[0], Node::Atom(a) if a == "_") {
                    None
                } else {
                    let mut v = Vec::new();
                    for s in syms_node {
                        let pair = as_list(s)?;
                        if pair.len() != 2 {
                            return Err("import symbol wants (name alias)".to_string());
                        }
                        v.push(ImportSymbol {
                            name: as_quoted(&pair[0])?.to_string(),
                            alias: parse_opt_string(&pair[1])?,
                        });
                    }
                    Some(v)
                };
            Ok(Stmt::Import {
                path: as_quoted(&items[1])?.to_string(),
                alias: parse_opt_string(&items[2])?,
                symbols,
            })
        }
        "extern" => {
            expect_arity(items, 3)?;
            let mut functions = Vec::new();
            for f in as_list(&items[3])? {
                let parts = as_list(f)?;
                if parts.len() != 3 {
                    return Err("extern fn wants (name (params) ret)".to_string());
                }
                let mut params = Vec::new();
                for p in as_list(&parts[1])? {
                    let pp = as_list(p)?;
                    if pp.len() != 2 {
                        return Err("extern param wants (name type)".to_string());
                    }
                    params.push(ExternParam {
                        name: as_quoted(&pp[0])?.to_string(),
                        param_type: parse_opt_string(&pp[1])?,
                    });
                }
                functions.push(ExternFnDecl {
                    name: as_quoted(&parts[0])?.to_string(),
                    params,
                    return_type: parse_opt_string(&parts[2])?,
                });
            }
            Ok(Stmt::ExternBlock {
                abi: as_quoted(&items[1])?.to_string(),
                lib: parse_opt_string(&items[2])?,
                functions,
            })
        }
        "say" => {
            expect_arity(items, 1)?;
            Ok(Stmt::Say(parse_expr(&items[1])?))
        }
        "let" => {
            expect_arity(items, 3)?;
            Ok(Stmt::Let {
                name: as_quoted(&items[1])?.to_string(),
                type_ann: parse_opt_string(&items[2])?,
                value: parse_expr(&items[3])?,
            })
        }
        "const" => {
            expect_arity(items, 2)?;
            Ok(Stmt::Const {
                name: as_quoted(&items[1])?.to_string(),
                value: parse_expr(&items[2])?,
            })
        }
        "assign" => {
            expect_arity(items, 2)?;
            Ok(Stmt::Assign {
                name: as_quoted(&items[1])?.to_string(),
                value: parse_expr(&items[2])?,
            })
        }
        "index-assign" => {
            expect_arity(items, 3)?;
            Ok(Stmt::IndexAssign {
                array: parse_expr(&items[1])?,
                index: parse_expr(&items[2])?,
                value: parse_expr(&items[3])?,
            })
        }
        "field-assign" => {
            expect_arity(items, 3)?;
            Ok(Stmt::FieldAssign {
                object: parse_expr(&items[1])?,
                field: as_quoted(&items[2])?.to_string(),
                value: parse_expr(&items[3])?,
            })
        }
        "struct" => {
            expect_arity(items, 5)?;
            Ok(Stmt::StructDef {
                name: as_quoted(&items[1])?.to_string(),
                fields: parse_quoted_seq(&items[2])?,
                field_types: parse_opt_str_seq(&items[3])?,
                defaults: parse_opt_expr_seq(&items[4])?,
                attributes: parse_attrs(&items[5])?,
            })
        }
        "enum" => {
            expect_arity(items, 2)?;
            let mut variants = Vec::new();
            for v in as_list(&items[2])? {
                let pair = as_list(v)?;
                if pair.len() != 2 {
                    return Err("enum variant wants (name expr)".to_string());
                }
                variants.push((as_quoted(&pair[0])?.to_string(), parse_opt_expr(&pair[1])?));
            }
            Ok(Stmt::EnumDef {
                name: as_quoted(&items[1])?.to_string(),
                variants,
            })
        }
        "if" => {
            expect_arity(items, 3)?;
            Ok(Stmt::If {
                condition: parse_expr(&items[1])?,
                then_block: parse_block(&items[2])?,
                else_block: parse_opt_block(&items[3])?,
            })
        }
        "while" => {
            expect_arity(items, 2)?;
            Ok(Stmt::While {
                condition: parse_expr(&items[1])?,
                body: parse_block(&items[2])?,
            })
        }
        "repeat" => {
            expect_arity(items, 1)?;
            Ok(Stmt::Repeat {
                body: parse_block(&items[1])?,
            })
        }
        "for" => {
            expect_arity(items, 5)?;
            Ok(Stmt::For {
                var: as_quoted(&items[1])?.to_string(),
                start: parse_expr(&items[2])?,
                end: parse_expr(&items[3])?,
                inclusive: parse_bool(&items[4])?,
                body: parse_block(&items[5])?,
            })
        }
        "foreach" => {
            expect_arity(items, 4)?;
            Ok(Stmt::ForEach {
                var: as_quoted(&items[1])?.to_string(),
                value_var: parse_opt_string(&items[2])?,
                iterable: parse_expr(&items[3])?,
                body: parse_block(&items[4])?,
            })
        }
        "function" => {
            if items.len() != 9 {
                return Err(format!("(function) wants 8 args, got {}", items.len() - 1));
            }
            Ok(Stmt::Function {
                name: as_quoted(&items[1])?.to_string(),
                params: parse_quoted_seq(&items[2])?,
                param_types: parse_opt_str_seq(&items[3])?,
                return_type: parse_opt_string(&items[4])?,
                defaults: parse_opt_expr_seq(&items[5])?,
                type_params: parse_quoted_seq(&items[6])?,
                attributes: parse_attrs(&items[7])?,
                body: parse_block(&items[8])?,
            })
        }
        "return" => {
            expect_arity(items, 1)?;
            Ok(Stmt::Return(parse_opt_expr(&items[1])?))
        }
        "break" => {
            expect_arity(items, 0)?;
            Ok(Stmt::Break)
        }
        "continue" => {
            expect_arity(items, 0)?;
            Ok(Stmt::Continue)
        }
        "expr" => {
            expect_arity(items, 1)?;
            Ok(Stmt::Expr(parse_expr(&items[1])?))
        }
        "throw" => {
            expect_arity(items, 1)?;
            Ok(Stmt::Throw(parse_opt_expr(&items[1])?))
        }
        "try" => {
            expect_arity(items, 4)?;
            Ok(Stmt::TryCatch {
                try_block: parse_block(&items[1])?,
                catch_var: parse_opt_string(&items[2])?,
                catch_block: parse_block(&items[3])?,
                finally_block: parse_opt_block(&items[4])?,
            })
        }
        "defer" => {
            expect_arity(items, 1)?;
            Ok(Stmt::Defer(Box::new(parse_stmt(&items[1])?)))
        }
        "pub" => {
            expect_arity(items, 1)?;
            Ok(Stmt::Pub(Box::new(parse_stmt(&items[1])?)))
        }
        "interface" => {
            expect_arity(items, 3)?;
            let mut methods = Vec::new();
            for m in as_list(&items[2])? {
                let parts = as_list(m)?;
                if parts.len() != 4 {
                    return Err("interface method wants (name (params) (types) ret)".to_string());
                }
                methods.push(InterfaceMethod {
                    name: as_quoted(&parts[0])?.to_string(),
                    params: parse_quoted_seq(&parts[1])?,
                    param_types: parse_opt_str_seq(&parts[2])?,
                    return_type: parse_opt_string(&parts[3])?,
                });
            }
            Ok(Stmt::InterfaceDef {
                name: as_quoted(&items[1])?.to_string(),
                methods,
                embedded: parse_quoted_seq(&items[3])?,
            })
        }
        _ => Err(format!("unknown stmt head {h:?}")),
    }
}

/// Parse canonical dump text (one S-expr per line) into a `Program`.
///
/// `0|0|error|...` diagnostic lines are rejected: they never reach the
/// reader in the pipeline (the driver checks the frontend exit first).
pub fn parse_program(text: &str) -> Result<Program, String> {
    for line in text.lines() {
        if line.starts_with("0|0|error|") {
            return Err("frontend error line, not a program".to_string());
        }
    }
    if text.trim().is_empty() {
        return Ok(Program { statements: vec![] });
    }
    let toks = tokenize(text)?;
    let mut pos = 0;
    let mut stmts = Vec::new();
    while pos < toks.len() {
        let node = parse_node(&toks, &mut pos)?;
        stmts.push(parse_stmt(&node).map_err(|e| format!("top-level: {e}"))?);
    }
    Ok(Program { statements: stmts })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::ast_sexpr::program_to_sexpr;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse_code(code: &str) -> Program {
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().expect("lex error");
        let mut parser = Parser::new(tokens);
        parser.parse_raw().expect("parse error")
    }

    fn roundtrip(code: &str) -> Program {
        let prog = parse_code(code);
        let dump = program_to_sexpr(&prog);
        let back =
            parse_program(&dump).unwrap_or_else(|e| panic!("reader failed: {e}\ndump: {dump}"));
        assert_eq!(prog, back, "AST mismatch for {code:?}");
        let dump2 = program_to_sexpr(&back);
        assert_eq!(dump, dump2, "re-dump mismatch for {code:?}");
        back
    }

    #[test]
    fn test_reader_say_arith() {
        roundtrip("say 1 + 2 * 3");
    }

    #[test]
    fn test_reader_float_bits() {
        // Shortest-roundtrip texts, inf/nan, negative zero.
        let prog = roundtrip("say 0.30000000000000004\nsay 1e308 * 10.0\nsay 0.0 * -1.0");
        let dump = program_to_sexpr(&prog);
        let back = parse_program(&dump).unwrap();
        assert_eq!(prog, back);
    }

    #[test]
    fn test_reader_float_specials() {
        for lit in ["inf", "-inf"] {
            let dump = format!("(say (float {lit}))");
            let prog = parse_program(&dump).unwrap();
            let f = match &prog.statements[0] {
                Stmt::Say(Expr::Float(f)) => *f,
                other => panic!("unexpected {other:?}"),
            };
            assert!(f.is_infinite() && (lit.starts_with('-') == f.is_sign_negative()));
            // Re-dump is byte-identical (same {:?} text).
            assert_eq!(program_to_sexpr(&prog), dump);
        }
        let prog = parse_program("(say (float NaN))").unwrap();
        match &prog.statements[0] {
            Stmt::Say(Expr::Float(f)) => assert!(f.is_nan()),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn test_reader_strings_unicode() {
        roundtrip("say \"héllo 😀\"\nsay \"a\\n\\\"q\\\"\\\\\"");
        // Surrogate-pair input decodes to the astral char.
        let prog = parse_program("(say (str \"\\ud83d\\ude00\"))").unwrap();
        match &prog.statements[0] {
            Stmt::Say(Expr::String(s)) => assert_eq!(s, "\u{1F600}"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn test_reader_all_stmt_shapes() {
        roundtrip("import \"std/str\" as str");
        roundtrip("from \"std/math\" import PI, floor as fl");
        roundtrip("say 1");
        roundtrip("let x: int = 5");
        roundtrip("const K = 2");
        roundtrip("x = 3");
        roundtrip("a[0] = 1");
        roundtrip("o.f = 2");
        roundtrip("struct P\nx: int = 1\nend");
        roundtrip("enum E\nA\nB = 2\nend");
        roundtrip("if a\nsay 1\nelif b\nsay 2\nelse\nsay 3\nend");
        roundtrip("while a\nsay 1\nend");
        roundtrip("repeat\nsay 1\nend");
        roundtrip("for i in 1..3\nsay i\nend");
        roundtrip("for ch in \"ab\"\nsay ch\nend");
        roundtrip("function f(x: int = 2) -> int\nreturn x\nend");
        roundtrip("return 1");
        roundtrip("break");
        roundtrip("continue");
        roundtrip("throw \"e\"");
        roundtrip("try\nsay 1\ncatch e\nsay 2\nfinally\nsay 3\nend");
        roundtrip("defer say 1");
        roundtrip("@deprecated(since = \"0.2\")\nfunction g()\nend");
        roundtrip("say a ?? b");
        roundtrip("say x is int");
        roundtrip("say x as int");
        roundtrip("say a?[0]");
        roundtrip("say Box { x: 1 }");
        roundtrip("say {\"k\": 1}");
        roundtrip("say f\"hi {x}\"");
        roundtrip("say true");
        roundtrip("say 'A'");
        roundtrip("say x not in y");
    }

    #[test]
    fn test_reader_rejects_error_lines() {
        assert!(parse_program("0|0|error|1:2").is_err());
        assert!(parse_program("(say (num 1)").is_err());
        assert!(parse_program("(wat)").is_err());
        assert!(parse_program("(say (num 99999999999999999999999999999999999999999))").is_err());
    }
}
