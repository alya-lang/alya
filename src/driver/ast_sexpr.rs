//! Canonical S-expression dump of the parse tree (`alya ast`).
//!
//! FORMAT (frozen for the self-host differential spike, `spike/selfhost/`):
//! - One line per top-level statement, `(program ...)` wrapper omitted in
//!   favor of streaming: each statement prints on its own line. Error
//!   files print `0|0|error|LINE:COL` and exit 1 (same contract as the
//!   lexer spike dump).
//! - All strings (identifiers, literals, type names, fields) print
//!   JSON-escaped and double-quoted, mirroring the lexer spike's
//!   `escape_cp` (`"` -> `\"`, `\` -> `\\`, control chars -> `\n` etc.,
//!   `<0x20`/`0x7F` -> `\u00XX`, non-ASCII -> `\uXXXX`/surrogates).
//!   The Alya port reuses that exact routine.
//! - Integers print decimal. Floats print Rust `{:?}` (shortest); the
//!   differential harness compares them by bit value, not text (same as
//!   the lexer float rule), so `1.0` vs `1` is not a diff.
//! - `None`/absent optionals print as `_`. Booleans as `true`/`false`.
//! - Parse-time desugars appear FOLDED (`true` -> `(num 1)`, `obj.m(a)`
//!   -> `(call "m" ((ident "obj") ...))`, slices -> `(call "slice" ...)`).
//!   The Alya port must replicate each desugar, not the surface syntax.
//! - `BinaryOp::NotIn` is never constructed by the parser (surface
//!   `x not in y` is `(unop not (binop in x y))`); the atom is reserved.
//!
//! Stage: the dump covers the RAW `parser.parse()` output, before import
//! resolution and default-arg/enum/const expansion. `elif`/`else if`
//! appear as a nested `(if ...)` in else position.

use crate::ast::{BinaryOp, Expr, Program, Stmt, UnaryOp};

/// JSON-escape one `char`, mirroring the lexer spike's `escape_cp`.
fn esc_char(cp: char, out: &mut String) {
    match cp {
        '"' => out.push_str("\\\""),
        '\\' => out.push_str("\\\\"),
        '\n' => out.push_str("\\n"),
        '\t' => out.push_str("\\t"),
        '\r' => out.push_str("\\r"),
        '\u{8}' => out.push_str("\\b"),
        '\u{c}' => out.push_str("\\f"),
        // Printable ASCII passes through raw (like the spike's escape_cp);
        // everything else is escaped.
        c if (c as u32) >= 32 && (c as u32) < 127 => out.push(c),
        c if (c as u32) < 32 || c as u32 == 127 => {
            out.push_str(&format!("\\u{:04x}", c as u32));
        }
        c if (c as u32) < 65536 => {
            out.push_str(&format!("\\u{:04x}", c as u32));
        }
        c => {
            // Non-BMP: surrogate pair, like the spike harness expects.
            let v = c as u32 - 65536;
            out.push_str(&format!(
                "\\u{:04x}\\u{:04x}",
                55296 + (v >> 10),
                56320 + (v & 1023)
            ));
        }
    }
}

/// JSON-escaped, double-quoted string atom.
fn qs(s: &str) -> String {
    // NUL cannot survive the Alya runtime (null-terminated strings); the
    // harness compares truncated at the first NUL, like the lexer diff.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for cp in s.chars() {
        if cp == '\0' {
            break;
        }
        esc_char(cp, &mut out);
    }
    out.push('"');
    out
}

fn opt_str(v: &Option<String>) -> String {
    match v {
        Some(s) => qs(s),
        None => "_".to_string(),
    }
}

fn binop_atom(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Modulo => "%",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::Greater => ">",
        BinaryOp::LessEqual => "<=",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::In => "in",
        BinaryOp::NotIn => "not-in",
        BinaryOp::Range => "..",
        BinaryOp::RangeInclusive => "..=",
    }
}

fn unop_atom(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Negate => "-",
        UnaryOp::Not => "not",
        UnaryOp::BitNot => "~",
    }
}

fn exprs_to_sexpr(es: &[Expr]) -> String {
    es.iter().map(expr_to_sexpr).collect::<Vec<_>>().join(" ")
}

pub fn expr_to_sexpr(e: &Expr) -> String {
    match e {
        Expr::Number(n) => format!("(num {})", n),
        Expr::Float(f) => format!("(float {:?})", f),
        Expr::String(s) => format!("(str {})", qs(s)),
        Expr::Identifier(n) => format!("(ident {})", qs(n)),
        Expr::Null => "(null)".to_string(),
        Expr::Binary { left, op, right } => format!(
            "(binop {} {} {})",
            binop_atom(op),
            expr_to_sexpr(left),
            expr_to_sexpr(right)
        ),
        Expr::Unary { op, expr } => {
            format!("(unop {} {})", unop_atom(op), expr_to_sexpr(expr))
        }
        Expr::Call { name, args } => {
            format!("(call {} ({}))", qs(name), exprs_to_sexpr(args))
        }
        Expr::InterpolatedString(parts) => {
            format!("(interp ({}))", exprs_to_sexpr(parts))
        }
        Expr::Array(es) => format!("(array ({}))", exprs_to_sexpr(es)),
        Expr::Index { array, index } => {
            format!("(index {} {})", expr_to_sexpr(array), expr_to_sexpr(index))
        }
        Expr::FieldAccess { object, field } => {
            format!("(field {} {})", expr_to_sexpr(object), qs(field))
        }
        Expr::StructInit { name, fields } => {
            let fs = fields
                .iter()
                .map(|(n, v)| format!("({} {})", qs(n), expr_to_sexpr(v)))
                .collect::<Vec<_>>()
                .join(" ");
            format!("(struct-init {} ({}))", qs(name), fs)
        }
        Expr::Map(kvs) => {
            let fs = kvs
                .iter()
                .map(|(k, v)| format!("({} {})", expr_to_sexpr(k), expr_to_sexpr(v)))
                .collect::<Vec<_>>()
                .join(" ");
            format!("(map ({}))", fs)
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "(ternary {} {} {})",
            expr_to_sexpr(condition),
            expr_to_sexpr(then_branch),
            expr_to_sexpr(else_branch)
        ),
        Expr::NullCoalesce { value, default } => format!(
            "(coalesce {} {})",
            expr_to_sexpr(value),
            expr_to_sexpr(default)
        ),
        Expr::OptionalFieldAccess { object, field } => {
            format!("(opt-field {} {})", expr_to_sexpr(object), qs(field))
        }
        Expr::OptionalIndex { array, index } => format!(
            "(opt-index {} {})",
            expr_to_sexpr(array),
            expr_to_sexpr(index)
        ),
        Expr::OptionalCall { callee, args } => {
            format!("(opt-call {} ({}))", qs(callee), exprs_to_sexpr(args))
        }
        Expr::ForceUnwrap(x) => format!("(unwrap {})", expr_to_sexpr(x)),
        Expr::TypeCheck {
            expr,
            target,
            negated,
        } => format!("(is {} {} {})", expr_to_sexpr(expr), qs(target), negated),
        Expr::Cast { expr, target } => {
            format!("(as {} {})", expr_to_sexpr(expr), qs(target))
        }
    }
}

fn block_to_sexpr(b: &[Stmt]) -> String {
    format!(
        "(block {})",
        b.iter().map(stmt_to_sexpr).collect::<Vec<_>>().join(" ")
    )
}

fn opt_block(b: &Option<Vec<Stmt>>) -> String {
    match b {
        Some(ss) => block_to_sexpr(ss),
        None => "_".to_string(),
    }
}

pub fn stmt_to_sexpr(s: &Stmt) -> String {
    match s {
        Stmt::Import {
            path,
            alias,
            symbols,
        } => {
            let syms = match symbols {
                Some(v) => v
                    .iter()
                    .map(|im| match &im.alias {
                        Some(a) => format!("({} {})", qs(&im.name), qs(a)),
                        None => format!("({} _)", qs(&im.name)),
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
                None => "_".to_string(),
            };
            format!("(import {} {} ({}))", qs(path), opt_str(alias), syms)
        }
        Stmt::ExternBlock {
            abi,
            lib,
            functions,
        } => {
            let fs = functions
                .iter()
                .map(|f| {
                    let ps = f
                        .params
                        .iter()
                        .map(|p| format!("({} {})", qs(&p.name), opt_str(&p.param_type)))
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!("({} ({}) {})", qs(&f.name), ps, opt_str(&f.return_type))
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("(extern {} {} ({}))", qs(abi), opt_str(lib), fs)
        }
        Stmt::Say(e) => format!("(say {})", expr_to_sexpr(e)),
        Stmt::Let {
            name,
            type_ann,
            value,
        } => format!(
            "(let {} {} {})",
            qs(name),
            opt_str(type_ann),
            expr_to_sexpr(value)
        ),
        Stmt::Const { name, value } => {
            format!("(const {} {})", qs(name), expr_to_sexpr(value))
        }
        Stmt::Assign { name, value } => {
            format!("(assign {} {})", qs(name), expr_to_sexpr(value))
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => format!(
            "(index-assign {} {} {})",
            expr_to_sexpr(array),
            expr_to_sexpr(index),
            expr_to_sexpr(value)
        ),
        Stmt::FieldAssign {
            object,
            field,
            value,
        } => format!(
            "(field-assign {} {} {})",
            expr_to_sexpr(object),
            qs(field),
            expr_to_sexpr(value)
        ),
        Stmt::StructDef {
            name,
            fields,
            field_types,
            defaults,
            attributes,
        } => format!(
            "(struct {} ({}) ({}) ({}) {})",
            qs(name),
            fields.iter().map(|f| qs(f)).collect::<Vec<_>>().join(" "),
            field_types
                .iter()
                .map(opt_str)
                .collect::<Vec<_>>()
                .join(" "),
            defaults
                .iter()
                .map(|d| d.as_ref().map(expr_to_sexpr).unwrap_or("_".to_string()))
                .collect::<Vec<_>>()
                .join(" "),
            attrs_to_sexpr(attributes),
        ),
        Stmt::EnumDef { name, variants } => {
            let vs = variants
                .iter()
                .map(|(n, v)| {
                    format!(
                        "({} {})",
                        qs(n),
                        v.as_ref().map(expr_to_sexpr).unwrap_or("_".to_string())
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("(enum {} ({}))", qs(name), vs)
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => format!(
            "(if {} {} {})",
            expr_to_sexpr(condition),
            block_to_sexpr(then_block),
            opt_block(else_block),
        ),
        Stmt::While { condition, body } => format!(
            "(while {} {})",
            expr_to_sexpr(condition),
            block_to_sexpr(body)
        ),
        Stmt::Repeat { body } => format!("(repeat {})", block_to_sexpr(body)),
        Stmt::For {
            var,
            start,
            end,
            inclusive,
            body,
        } => format!(
            "(for {} {} {} {} {})",
            qs(var),
            expr_to_sexpr(start),
            expr_to_sexpr(end),
            inclusive,
            block_to_sexpr(body)
        ),
        Stmt::ForEach {
            var,
            value_var,
            iterable,
            body,
        } => format!(
            "(foreach {} {} {} {})",
            qs(var),
            opt_str(value_var),
            expr_to_sexpr(iterable),
            block_to_sexpr(body)
        ),
        Stmt::Function {
            name,
            params,
            param_types,
            return_type,
            defaults,
            body,
            type_params,
            attributes,
        } => format!(
            "(function {} ({}) ({}) {} ({}) ({}) {} {})",
            qs(name),
            params.iter().map(|p| qs(p)).collect::<Vec<_>>().join(" "),
            param_types
                .iter()
                .map(opt_str)
                .collect::<Vec<_>>()
                .join(" "),
            opt_str(return_type),
            defaults
                .iter()
                .map(|d| d.as_ref().map(expr_to_sexpr).unwrap_or("_".to_string()))
                .collect::<Vec<_>>()
                .join(" "),
            type_params
                .iter()
                .map(|t| qs(t))
                .collect::<Vec<_>>()
                .join(" "),
            attrs_to_sexpr(attributes),
            block_to_sexpr(body),
        ),
        Stmt::Return(e) => format!(
            "(return {})",
            e.as_ref().map(expr_to_sexpr).unwrap_or("_".to_string())
        ),
        Stmt::Break => "(break)".to_string(),
        Stmt::Continue => "(continue)".to_string(),
        Stmt::Expr(e) => format!("(expr {})", expr_to_sexpr(e)),
        Stmt::Throw(e) => format!(
            "(throw {})",
            e.as_ref().map(expr_to_sexpr).unwrap_or("_".to_string())
        ),
        Stmt::TryCatch {
            try_block,
            catch_var,
            catch_block,
            finally_block,
        } => format!(
            "(try {} {} {} {})",
            block_to_sexpr(try_block),
            opt_str(catch_var),
            block_to_sexpr(catch_block),
            opt_block(finally_block),
        ),
        Stmt::Defer(s) => format!("(defer {})", stmt_to_sexpr(s)),
        Stmt::Pub(s) => format!("(pub {})", stmt_to_sexpr(s)),
        Stmt::InterfaceDef {
            name,
            methods,
            embedded,
        } => {
            let ms = methods
                .iter()
                .map(|m| {
                    format!(
                        "({} ({}) ({}) {})",
                        qs(&m.name),
                        m.params.iter().map(|p| qs(p)).collect::<Vec<_>>().join(" "),
                        m.param_types
                            .iter()
                            .map(opt_str)
                            .collect::<Vec<_>>()
                            .join(" "),
                        opt_str(&m.return_type),
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "(interface {} ({}) ({}))",
                qs(name),
                ms,
                embedded.iter().map(|e| qs(e)).collect::<Vec<_>>().join(" "),
            )
        }
    }
}

fn attrs_to_sexpr(attrs: &[crate::ast::Attribute]) -> String {
    let ms = attrs
        .iter()
        .map(|a| {
            let args = a
                .args
                .iter()
                .map(|(k, v)| match k {
                    Some(n) => format!("({} {})", qs(n), qs(v)),
                    None => format!("(_ {})", qs(v)),
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("({} ({}))", qs(&a.name), args)
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("({})", ms)
}

/// Canonical dump: one S-expression per top-level statement, one per line.
pub fn program_to_sexpr(p: &Program) -> String {
    p.statements
        .iter()
        .map(stmt_to_sexpr)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse_code(code: &str) -> Program {
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().expect("lex error");
        let mut parser = Parser::new(tokens);
        parser.parse_raw().expect("parse error")
    }

    #[test]
    fn test_sexpr_left_assoc_add() {
        // 1+2*3 parses as 1+(2*3): factor binds tighter than term.
        let p = parse_code("say 1 + 2 * 3");
        assert_eq!(
            program_to_sexpr(&p),
            "(say (binop + (num 1) (binop * (num 2) (num 3))))"
        );
    }

    #[test]
    fn test_sexpr_null_coalesce_left_assoc() {
        // a??b??c is LEFT-nested (loop with parse_or on the right).
        let p = parse_code("say a ?? b ?? c");
        assert_eq!(
            program_to_sexpr(&p),
            "(say (coalesce (coalesce (ident \"a\") (ident \"b\")) (ident \"c\")))"
        );
    }

    #[test]
    fn test_sexpr_if_elif_nesting() {
        // elif folds into a nested if in else position.
        let p = parse_code("if a\nsay 1\nelif b\nsay 2\nelse\nsay 3\nend");
        assert_eq!(
            program_to_sexpr(&p),
            "(if (ident \"a\") (block (say (num 1))) (block (if (ident \"b\") (block (say (num 2))) (block (say (num 3))))))"
        );
    }

    #[test]
    fn test_sexpr_true_rune_desugar() {
        // Parse-time desugars appear folded in the dump.
        let p = parse_code("say true\nsay 'A'");
        assert_eq!(program_to_sexpr(&p), "(say (num 1))\n(say (num 65))");
    }
}
