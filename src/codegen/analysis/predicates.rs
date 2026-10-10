use crate::ast::*;
use crate::codegen::context::VarType;
use crate::codegen::kinds::{
    kind_of_literal, KIND_ARRAY, KIND_FLOAT, KIND_INT, KIND_MAP, KIND_STRING, KIND_STRUCT,
    KIND_UNKNOWN,
};
use std::collections::{HashMap, HashSet};

/// Resolves the struct type of a method-call receiver using only the
/// variable table (mirrors `CodeGen::get_expr_struct_name` for the
/// shapes method receivers take: identifiers, same-type arithmetic,
/// chained calls, and struct field paths).
fn receiver_struct_name(expr: &Expr, vars: &HashMap<String, VarType>) -> Option<String> {
    match expr {
        Expr::Identifier(id) => match vars.get(id) {
            Some(VarType::Struct { struct_name, .. }) => Some(struct_name.clone()),
            _ => None,
        },
        Expr::Binary {
            left,
            op:
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Modulo,
            ..
        } => {
            let s = receiver_struct_name(left, vars)?;
            let bare_s = s.rsplit("::").next().unwrap_or(&s);
            let bare_s = bare_s.rsplit("__").next().unwrap_or(bare_s);
            let sym = match expr {
                Expr::Binary {
                    op: BinaryOp::Add, ..
                } => "+",
                Expr::Binary {
                    op: BinaryOp::Subtract,
                    ..
                } => "-",
                Expr::Binary {
                    op: BinaryOp::Multiply,
                    ..
                } => "*",
                Expr::Binary {
                    op: BinaryOp::Divide,
                    ..
                } => "/",
                _ => "%",
            };
            for cand in [
                format!("{}__operator{}", s, sym),
                format!("{}__operator{}", bare_s, sym),
            ] {
                if let Some(VarType::Struct { struct_name, .. }) =
                    vars.get(&format!("fn_ret_struct:{}", cand))
                {
                    return Some(struct_name.clone());
                }
            }
            None
        }
        Expr::Call { name, args } => {
            if let Some(first) = args.first() {
                if let Some(s) = receiver_struct_name(first, vars) {
                    let bare_s = s.rsplit("::").next().unwrap_or(&s);
                    let bare_s = bare_s.rsplit("__").next().unwrap_or(bare_s);
                    let bare_c = name.rsplit("::").next().unwrap_or(name.as_str());
                    let bare_c = bare_c.rsplit("__").next().unwrap_or(bare_c);
                    for ss in [s.as_str(), bare_s] {
                        for cc in [name.as_str(), bare_c] {
                            for key in [
                                format!("fn_ret_struct:{}__{}", ss, cc),
                                format!("fn_ret_struct:{}::{}", ss, cc),
                            ] {
                                if let Some(VarType::Struct { struct_name, .. }) = vars.get(&key) {
                                    return Some(struct_name.clone());
                                }
                            }
                        }
                    }
                }
            }
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            for key in [
                format!("fn_ret_struct:{}", name),
                format!("fn_ret_struct:{}", bare),
            ] {
                if let Some(VarType::Struct { struct_name, .. }) = vars.get(&key) {
                    return Some(struct_name.clone());
                }
            }
            None
        }
        Expr::FieldAccess { object, field } => {
            let p = receiver_struct_name(object, vars)?;
            let bare_p = p.rsplit("::").next().unwrap_or(&p);
            let bare_p = bare_p.rsplit("__").next().unwrap_or(bare_p);
            for key in [
                format!("struct_field_struct:{}.{}", p, field),
                format!("struct_field_struct:{}.{}", bare_p, field),
            ] {
                if let Some(VarType::Struct { struct_name, .. }) = vars.get(&key) {
                    return Some(struct_name.clone());
                }
            }
            None
        }
        Expr::ForceUnwrap(inner) => receiver_struct_name(inner, vars),
        _ => None,
    }
}

/// Whether a function-name spelling is simple (no `::` or `__`
/// qualification). Used by alya-lang/alya#101 consult gates: a
/// qualified-spelled call must never consult bare markers (markers set
/// by an unrelated same-bare function would be inherited and callers
/// silently take the wrong path — observed: int calls read as float on
/// arm64). Single-segment names behave exactly as before.
pub fn is_simple_name(name: &str) -> bool {
    !name.contains("::") && !name.contains("__")
}

/// Whether a bare call with this name must resolve to the runtime builtin
/// and never to a same-named struct method via the single-method fallback
/// (alya-lang/alya#144): the runtime already provides `fn_<name>`, so the
/// fallback cannot fix a link error — it can only hijack the builtin call
/// (observed: `len(chunk)` on a dynamic array bound to `Buf__len`, reading
/// `.total` off the array and running past its end).
/// Only names with a confirmed runtime `fn_<name>` target that plausibly
/// double as method names are listed; anything else keeps the legacy
/// fallback (a loud link error if truly missing).
pub fn builtin_blocks_method_fallback(name: &str) -> bool {
    let bare = name.rsplit("::").next().unwrap_or(name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    matches!(
        bare,
        "len"
            | "length"
            | "byte_length"
            | "keys"
            | "values"
            | "contains"
            | "has"
            | "get"
            | "set"
            | "remove"
            | "slice"
            | "split"
            | "join"
            | "trim"
            | "upper"
            | "lower"
            | "substr"
            | "substring"
            | "bytes"
            | "str"
            | "ord"
            | "chr"
    )
}

/// Sentinel prefix marking an ambiguous bare function name
/// (alya-lang/alya#101): two or more functions share the same bare
/// (`inner` + `outer__inner`, `S__m` + top-level `m`). A bare marker
/// recorded by one of them would be inherited by callers of the other,
/// so recording sites skip bare inserts for ambiguous bares (the
/// affected calls degrade to dynamic dispatch, always sound) while
/// qualified-exact markers stay precise. Unique bares behave exactly
/// as before.
///
/// The sentinel travels inside the marker sets themselves
/// (`fn_ambiguous:<bare>` in `known_*` sets and `ctx.variables`), so
/// recording sites need no signature changes. `$`-mangled hoisted
/// nested functions (#99) never collide: `$` survives both `::`- and
/// `__`-stripping, so each keeps a distinct bare.
pub fn ambiguous_bare_key(bare: &str) -> String {
    format!("fn_ambiguous:{}", bare)
}

/// Bare names shared by more than one function in the program:
/// top-level definitions plus extern declarations (both participate
/// in bare lookups).
pub fn ambiguous_bares_of_program(program: &Program) -> HashSet<String> {
    ambiguous_bares_of_stmts(&program.statements)
}

/// [`ambiguous_bares_of_program`] over a statement slice (for passes
/// that only see statements, e.g. freshness inference).
pub fn ambiguous_bares_of_stmts(statements: &[Stmt]) -> HashSet<String> {
    use std::collections::HashMap as CountMap;
    fn bare_of(name: &str) -> &str {
        let bare = name.rsplit("::").next().unwrap_or(name);
        bare.rsplit("__").next().unwrap_or(bare)
    }
    let mut counts: CountMap<String, usize> = CountMap::new();
    for stmt in statements {
        let stmt = stmt.inner_stmt();
        if let Stmt::Function { name, .. } = stmt {
            *counts.entry(bare_of(name).to_string()).or_insert(0) += 1;
        }
        if let Stmt::ExternBlock { functions, .. } = stmt {
            for f in functions {
                *counts.entry(bare_of(&f.name).to_string()).or_insert(0) += 1;
            }
        }
    }
    counts
        .into_iter()
        .filter_map(|(bare, n)| if n > 1 { Some(bare) } else { None })
        .collect()
}

/// Insert one `fn_ambiguous:<bare>` sentinel per ambiguous bare.
/// Call once per marker set at construction.
pub fn seed_ambiguity_markers(program: &Program, set: &mut HashSet<String>) {
    for bare in ambiguous_bares_of_program(program) {
        set.insert(ambiguous_bare_key(&bare));
    }
}

/// True when `name` may seed bare markers: simple spellings always
/// (the exact key IS the bare key), qualified spellings only when no
/// other function shares the bare.
///
/// A macro (not a function) so call sites compile uniformly whether
/// their set binding is owned, `&mut`, or `&`: the expanded method
/// call auto-refs either way.
#[macro_export]
macro_rules! may_record_bare {
    ($set:expr, $name:expr, $bare:expr) => {
        $crate::codegen::analysis::predicates::is_simple_name($name)
            || !$set.contains(&$crate::codegen::analysis::ambiguous_bare_key($bare))
    };
}

/// `HashMap` (`ctx.variables`) variant of [`may_record_bare`].
pub fn may_record_bare_vars(vars: &HashMap<String, VarType>, name: &str, bare: &str) -> bool {
    is_simple_name(name) || !vars.contains_key(&ambiguous_bare_key(bare))
}

/// Whether a stored string value needs an immortal stable-region copy
/// (`alya_str_store`) instead of the ring-buffer pointer as produced.
/// String literals live in rodata (immortal already); every other string
/// value may sit in the wrapping ring buffer, so named stores (variables,
/// container slots, struct fields) duplicate it. Transient uses (say,
/// conditions, call args, returns) keep the ring pointer.
/// Explicitly heap-owned calls (`str_clone`, `string_clone`, paired with
/// `str_free`) are excluded: duplicating them would break the free pairing
/// (freeing a stable-region copy corrupts the heap).
pub fn string_store_needs_dup(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    if let Expr::Call { name, .. } = expr {
        let bare = name.rsplit("::").next().unwrap_or(name.as_str());
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        if bare == "str_clone" || bare == "string_clone" {
            return false;
        }
    }
    is_string_expr(expr, vars) && !matches!(expr, Expr::String(_))
}

/// Returns true if `expr` syntactically references variable `target`.
/// Used to identify self-accumulating loop variables (`res += ...`, `res = res + ...`).
pub fn expr_mentions_var(expr: &Expr, target: &str) -> bool {
    let bare_target = target.rsplit("::").next().unwrap_or(target);
    let bare_target = bare_target.rsplit("__").next().unwrap_or(bare_target);
    match expr {
        Expr::Identifier(id) => {
            let bare_id = id.rsplit("::").next().unwrap_or(id.as_str());
            let bare_id = bare_id.rsplit("__").next().unwrap_or(bare_id);
            id == target || id == bare_target || bare_id == target || bare_id == bare_target
        }
        Expr::Binary { left, right, .. } => {
            expr_mentions_var(left, target) || expr_mentions_var(right, target)
        }
        Expr::Unary { expr: inner, .. } => expr_mentions_var(inner, target),
        Expr::Call { args, .. } => args.iter().any(|a| expr_mentions_var(a, target)),
        Expr::OptionalCall { args, .. } => args.iter().any(|a| expr_mentions_var(a, target)),
        Expr::Index { array, index } => {
            expr_mentions_var(array, target) || expr_mentions_var(index, target)
        }
        Expr::OptionalIndex { array, index } => {
            expr_mentions_var(array, target) || expr_mentions_var(index, target)
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            expr_mentions_var(object, target)
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_mentions_var(condition, target)
                || expr_mentions_var(then_branch, target)
                || expr_mentions_var(else_branch, target)
        }
        Expr::NullCoalesce { value, default } => {
            expr_mentions_var(value, target) || expr_mentions_var(default, target)
        }
        Expr::Array(elements) | Expr::InterpolatedString(elements) => {
            elements.iter().any(|e| expr_mentions_var(e, target))
        }
        Expr::StructInit { fields, .. } => {
            fields.iter().any(|(_, val)| expr_mentions_var(val, target))
        }
        Expr::Map(pairs) => pairs
            .iter()
            .any(|(k, v)| expr_mentions_var(k, target) || expr_mentions_var(v, target)),
        Expr::ForceUnwrap(inner) | Expr::TypeCheck { expr: inner, .. } => {
            expr_mentions_var(inner, target)
        }
        Expr::Cast { expr: inner, .. } => expr_mentions_var(inner, target),
        _ => false,
    }
}

/// Whether an integer expression carries unsigned (u64-family) semantics
/// (B4): explicit `u64`/`u32`/`uint`/`usize` annotations (locals, globals,
/// params), `as u64`-family casts, over-i64 literals, or binary/unary
/// compositions thereof (unsigned contaminates, C-promotion style).
/// Everything else is signed (status quo): computed/dynamic values without
/// an unsigned root keep today's behavior exactly.
pub fn is_unsigned_expr(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Number(n) => *n > i64::MAX as i128,
        Expr::Identifier(name) => vars.contains_key(&format!("var_is_uint:{}", name)),
        Expr::Cast { target, .. } => {
            let t = target.to_lowercase();
            t == "u64" || t == "uint" || t == "u32" || t == "usize" || t == "u8" || t == "u16"
        }
        Expr::Binary { left, right, .. } => {
            is_unsigned_expr(left, vars) || is_unsigned_expr(right, vars)
        }
        Expr::Unary { expr: inner, .. } => is_unsigned_expr(inner, vars),
        Expr::ForceUnwrap(inner) => is_unsigned_expr(inner, vars),
        _ => false,
    }
}

pub fn is_string_expr(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::String(_) => true,
        Expr::InterpolatedString(_) => true,
        Expr::Call { name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if matches!(
                bare,
                "ask"
                    | "str"
                    | "trim"
                    | "upper"
                    | "lower"
                    | "to_upper"
                    | "to_lower"
                    | "substring"
                    | "substr"
                    | "join"
                    | "char_at"
                    | "chr"
                    | "char_from_code"
                    | "read_file"
                    | "get_env"
                    | "env"
                    | "env_or"
                    | "target_os"
                    | "target_arch"
                    | "os_name"
                    | "arch_name"
                    | "arch"
                    | "platform"
                    | "temp_dir"
                    | "home_dir"
                    | "user_name"
                    | "hostname"
                    | "null_device"
                    | "path_list_separator"
                    | "arg_at"
                    | "str_from_ptr"
                    | "replace"
                    | "str_repeat"
                    | "pad_left"
                    | "pad_right"
                    | "center"
                    | "trim_start"
                    | "ltrim"
                    | "trim_end"
                    | "rtrim"
                    | "trim_char"
                    | "capitalize"
                    | "title_case"
                    | "reverse_str"
                    | "truncate"
                    | "slugify"
                    | "path_separator"
                    | "path_join"
                    | "file_name"
                    | "file_ext"
                    | "parent_dir"
                    | "file_stem"
                    | "base64_encode"
                    | "base64_decode"
                    | "to_base64"
                    | "from_base64"
                    | "hex_encode"
                    | "hex_decode"
                    | "to_hex"
                    | "from_hex"
                    | "json_object"
                    | "json_map"
                    | "json_string"
                    | "json_escape"
                    | "json_null"
                    | "json_int"
                    | "json_float"
                    | "json_kv"
                    | "read_file_or"
                    | "glob_escape"
                    | "str_clone"
                    | "string_clone"
                    | "tcp_recv"
                    | "net_recv"
                    | "http_recv"
                    | "net_udp_recv"
                    | "udp_recv"
                    | "net_peer_ip"
                    | "tcp_peer_ip"
                    | "tcp_peer_addr"
                    | "get_cwd"
                    | "cwd"
                    | "__native_get_cwd"
                    | "format_iso"
                    | "format_date"
                    | "format_time_hhmmss"
                    | "month_name"
                    | "month_short_name"
                    | "weekday_name"
                    | "weekday_short_name"
                    | "iso_now"
                    | "date_now"
                    | "time_now"
                    | "file_basename"
                    | "file_extension"
                    | "file_parent"
                    | "console_prompt"
                    | "http_status_text"
                    | "basic_auth"
                    | "format_binary"
                    | "to_string"
                    | "typeof"
            ) || bare.starts_with("__alya_format:")
            {
                return true;
            }
            if matches!(
                bare,
                "int"
                    | "float"
                    | "len"
                    | "arr_len"
                    | "ord"
                    | "time"
                    | "clock_ms"
                    | "rand"
                    | "rand_int"
                    | "abs"
                    | "abs_val"
            ) {
                return false;
            }
            if bare == "slice" {
                return !args.is_empty() && is_string_expr(&args[0], vars);
            }
            if bare == "get" && args.len() >= 3 && is_string_expr(&args[2], vars) {
                return true;
            }
            if let Some(first_arg) = args.first() {
                let struct_name = match first_arg {
                    Expr::Identifier(id) => match vars.get(id) {
                        Some(VarType::Struct { struct_name, .. }) => Some(struct_name.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(sname) = struct_name {
                    let bare_s = sname.rsplit("::").next().unwrap_or(&sname);
                    let bare_s = bare_s.rsplit("__").next().unwrap_or(bare_s);
                    let c1 = format!("{}__{}", sname, name);
                    let c2 = format!("{}__{}", bare_s, name);
                    let suffix1 = format!("__{}", c1);
                    let suffix2 = format!("__{}", c2);
                    if vars.contains_key(&format!("fn_ret_str:{}", c1))
                        || vars.contains_key(&format!("fn_ret_str:{}", c2))
                        || vars.keys().any(|k| {
                            k.starts_with("fn_ret_str:")
                                && (k.ends_with(&suffix1) || k.ends_with(&suffix2))
                        })
                    {
                        return true;
                    }
                    // The receiver is a known struct instance, so this is a
                    // method call: an explicit integer return annotation on
                    // the method authoritatively rules out `string`, even
                    // when an unrelated same-named plain function (e.g.
                    // `touch(path)`) left bare `fn_ret_str:` markers behind.
                    // Without such a marker, fall through to the bare
                    // markers below (status quo for unannotated methods).
                    if vars.contains_key(&format!("fn_ret_int:{}", c1))
                        || vars.contains_key(&format!("fn_ret_int:{}", c2))
                        || vars.keys().any(|k| {
                            k.starts_with("fn_ret_int:")
                                && (k.ends_with(&suffix1) || k.ends_with(&suffix2))
                        })
                    {
                        return false;
                    }
                }
            }
            if bare == "recv" || bare == "try_recv" {
                if let Some(first_arg) = args.first() {
                    let ch_name = match first_arg {
                        Expr::Identifier(id) => Some(id.as_str()),
                        _ => None,
                    };
                    if let Some(ch) = ch_name {
                        if vars.contains_key(ch) {
                            return vars.contains_key(&format!("channel_elem_str:{}", ch));
                        }
                    }
                }
            }
            if matches!(
                bare,
                "json_parse_array"
                    | "parse_array"
                    | "json_parse_object"
                    | "parse_object"
                    | "json_parse"
                    | "parse"
                    | "_json_parse_val"
            ) {
                return false;
            }
            // A callee known to return a struct is never a string, so
            // bare-suffix `fn_ret_str` hits from unrelated same-named
            // methods (e.g. `CliContext.command` vs the `command`
            // facade) must not apply (the struct would be copied as
            // string bytes by `str_store`).
            if call_returns_struct(name, vars) {
                return false;
            }
            vars.contains_key(&format!("fn_ret_str:{}", name))
                // #101: qualified-spelled calls consult exact markers
                // only; bare/suffix fallbacks would inherit markers set
                // by an unrelated same-bare function.
                || (is_simple_name(name)
                    && (vars.contains_key(&format!("fn_ret_str:{}", bare))
                        || vars.keys().any(|k| {
                            k.starts_with("fn_ret_str:")
                                && (k.ends_with(&format!("__{}", bare))
                                    || k.ends_with(&format!("::{}", bare)))
                        })))
        }
        Expr::Identifier(name) => {
            if let Some(var_type) = vars.get(name) {
                matches!(var_type, VarType::StringLabel(_) | VarType::StringOffset(_))
            } else {
                false
            }
        }
        Expr::Index { array, index } => {
            if let Expr::String(field) = &**index {
                // Key-level veto (alya-lang/alya#138): a non-string
                // value stored under this key in any map voids the
                // bare claim, which is map-agnostic.
                if !vars.contains_key(&format!("map_nonstr_key:{}", field))
                    && vars.contains_key(&format!("map_field_str:{}", field))
                {
                    return true;
                }
            }
            if let (Expr::Identifier(obj_name), Expr::String(field)) = (&**array, &**index) {
                let key = format!("map_str:{}.{}", obj_name, field);
                if !vars.contains_key(&format!("map_nonstr_key:{}", field))
                    && vars.contains_key(&key)
                {
                    return true;
                }
            }
            if matches!(**index, Expr::String(_)) || is_string_expr(index, vars) {
                return false;
            }
            if let (Expr::Identifier(arr_name), Expr::Number(idx)) = (&**array, &**index) {
                let key = format!("tuple_elem_str:{}:{}", arr_name, *idx as usize);
                if vars.contains_key(&key) {
                    return true;
                }
            }
            is_string_array(array, vars) || is_string_expr(array, vars)
        }
        Expr::FieldAccess { object, field } => {
            if let Expr::Identifier(obj_name) = &**object {
                let key = format!("{}.{}", obj_name, field);
                if let Some(var_type) = vars.get(&key) {
                    if matches!(var_type, VarType::StringLabel(_) | VarType::StringOffset(_)) {
                        return true;
                    }
                }
                if let Some(VarType::Struct { struct_name, .. }) = vars.get(obj_name) {
                    let bare = struct_name.rsplit("::").next().unwrap_or(struct_name);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    let field_key = format!("struct_field_str:{}.{}", struct_name, field);
                    let bare_key = format!("struct_field_str:{}.{}", bare, field);
                    return vars.contains_key(&field_key) || vars.contains_key(&bare_key);
                }
            }
            let global_field_key = format!("struct_field_str:{}", field);
            if !struct_field_kind_mixed(vars, field) && vars.contains_key(&global_field_key) {
                return true;
            }
            false
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add,
            right,
        } => is_string_expr(left, vars) || is_string_expr(right, vars),
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => {
            // Both arms must be strings: with mixed arms the result is
            // dynamic, and callers (e.g. `str()` elision) must not assume
            // a string outcome from a single string arm. Null arms are
            // transparent (a `when` without `else` desugars to Null).
            (is_string_expr(then_branch, vars) || is_null_expr(then_branch, vars))
                && (is_string_expr(else_branch, vars) || is_null_expr(else_branch, vars))
                && (is_string_expr(then_branch, vars) || is_string_expr(else_branch, vars))
        }
        Expr::NullCoalesce { value, default } => {
            // Same null-transparent rule: `x ?? "lit"` is string-like
            // unless a non-null arm proves otherwise.
            (is_string_expr(value, vars) || is_null_expr(value, vars))
                && (is_string_expr(default, vars) || is_null_expr(default, vars))
                && (is_string_expr(value, vars) || is_string_expr(default, vars))
        }
        Expr::OptionalFieldAccess { object, field } => is_string_expr(
            &Expr::FieldAccess {
                object: object.clone(),
                field: field.clone(),
            },
            vars,
        ),
        Expr::OptionalIndex { array, index } => is_string_expr(
            &Expr::Index {
                array: array.clone(),
                index: index.clone(),
            },
            vars,
        ),
        Expr::OptionalCall { callee, args } => is_string_expr(
            &Expr::Call {
                name: callee.clone(),
                args: args.clone(),
            },
            vars,
        ),
        Expr::Cast {
            expr: inner,
            target,
        } => {
            let t = target.to_lowercase();
            if t == "int" || t == "i64" || t == "float" || t == "f64" || t == "bool" || t == "rune"
            {
                false
            } else {
                t == "string" || t == "str" || is_string_expr(inner, vars)
            }
        }
        Expr::ForceUnwrap(inner) => is_string_expr(inner, vars),
        _ => false,
    }
}

pub fn is_array_expr(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Array(_) => true,
        Expr::Identifier(name) => {
            matches!(vars.get(name), Some(VarType::Array(_)))
        }
        Expr::Call { name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            matches!(
                bare,
                "split"
                    | "args"
                    | "cli_args"
                    | "keys"
                    | "values"
                    | "lines"
                    | "read_lines"
                    | "set_to_array"
                    | "stack_new"
                    | "queue_new"
                    | "queue_to_array"
                    | "deque_new"
                    | "pq_new"
                    | "priority_queue_new"
                    | "array_slice"
                    | "array_clone"
                    | "array_concat"
                    | "array_reverse"
                    | "array_reverse_in_place"
                    | "array_unique"
                    | "array_sort"
                    | "array_sort_in_place"
                    | "array_chunk"
                    | "array_fill"
                    | "map_entries"
                    | "rand_sample"
                    | "rand_shuffle"
                    | "rand_shuffled"
                    | "csv_parse"
                    | "csv_parse_with_delimiter"
                    | "csv_parse_tsv"
                    | "tsv_parse"
                    | "csv_parse_records"
                    | "csv_parse_records_with_delimiter"
                    | "tsv_parse_records"
                    | "csv_read_file"
                    | "csv_read_file_with_delimiter"
                    | "csv_read_records"
                    | "csv_read_records_with_delimiter"
                    | "tsv_read_file"
                    | "tsv_read_records"
                    | "url_path_segments"
                    | "glob"
                    | "glob_dir"
                    | "glob_filter"
                    | "list_dir"
                    | "read_dir"
                    | "fs_list_dir"
                    | "fs_read_dir"
                    | "list_dir_recursive"
                    | "fs_list_dir_recursive"
                    | "tcp_recv_bytes"
                    | "net_recv_bytes"
                    | "udp_recv_bytes"
                    | "net_udp_recv_bytes"
                    | "read_bytes" // NOTE: `json_parse_array`, `parse_array` are deliberately
                                           // absent: they forward to the dynamic JSON parser (any
                                           // value kind), so a static array claim miscompiles.
            ) || (bare == "slice" && !args.is_empty() && is_array_expr(&args[0], vars))
                || vars.contains_key(&format!("fn_ret_arr:{}", name))
                // #101: qualified-spelled calls consult exact markers only.
                || (is_simple_name(name)
                    && (vars.contains_key(&format!("fn_ret_arr:{}", bare))
                        || vars.contains_key(&format!("fn_ret_str_arr:{}", bare))))
                || vars.contains_key(&format!("fn_ret_arr:{}", name.replace("::", "__")))
                || vars.contains_key(&format!("fn_ret_arr:{}", name.replace("__", "::")))
                || vars.contains_key(&format!("fn_ret_str_arr:{}", name))
                || vars.contains_key(&format!("fn_ret_str_arr:{}", name.replace("::", "__")))
                || vars.contains_key(&format!("fn_ret_str_arr:{}", name.replace("__", "::")))
        }
        Expr::FieldAccess { object, field } => {
            if let Expr::Identifier(obj_name) = &**object {
                let key = format!("{}.{}", obj_name, field);
                if let Some(var_type) = vars.get(&key) {
                    if matches!(var_type, VarType::Array(_)) {
                        return true;
                    }
                }
                if let Some(VarType::Struct { struct_name, .. }) = vars.get(obj_name) {
                    let field_key = format!("struct_field_arr:{}.{}", struct_name, field);
                    if vars.contains_key(&field_key) {
                        return true;
                    }
                }
            }
            let global_field_key = format!("struct_field_arr:{}", field);
            if !struct_field_kind_mixed(vars, field) && vars.contains_key(&global_field_key) {
                return true;
            }
            false
        }
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => {
            // Null-transparent AND (see is_string_expr): both non-null arms
            // must be arrays for callers to assume an array outcome.
            (is_array_expr(then_branch, vars) || is_null_expr(then_branch, vars))
                && (is_array_expr(else_branch, vars) || is_null_expr(else_branch, vars))
                && (is_array_expr(then_branch, vars) || is_array_expr(else_branch, vars))
        }
        Expr::NullCoalesce { value, default } => {
            (is_array_expr(value, vars) || is_null_expr(value, vars))
                && (is_array_expr(default, vars) || is_null_expr(default, vars))
                && (is_array_expr(value, vars) || is_array_expr(default, vars))
        }
        Expr::OptionalFieldAccess { object, field } => is_array_expr(
            &Expr::FieldAccess {
                object: object.clone(),
                field: field.clone(),
            },
            vars,
        ),
        Expr::OptionalIndex { array, index } => is_array_expr(
            &Expr::Index {
                array: array.clone(),
                index: index.clone(),
            },
            vars,
        ),
        Expr::OptionalCall { callee, args } => is_array_expr(
            &Expr::Call {
                name: callee.clone(),
                args: args.clone(),
            },
            vars,
        ),
        Expr::ForceUnwrap(inner) => is_array_expr(inner, vars),
        _ => false,
    }
}

pub fn is_map_expr(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Identifier(name) => {
            matches!(vars.get(name), Some(VarType::Map(_)))
        }
        Expr::Call { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            matches!(
                bare,
                "map"
                    | "set_new"
                    | "set_from_array"
                    | "set_union"
                    | "set_intersection"
                    | "set_difference"
                    | "map_clone"
                    | "map_merge"
                    | "map_from_entries"
                    | "url_parse_query" // NOTE: `json_parse`, `json_parse_object`, `parse_object`
                                        // are deliberately absent: they forward to the dynamic
                                        // JSON parser (any value kind), so a static map claim
                                        // miscompiles array/string results (for-loop vars, `+=`).
            ) || vars.contains_key(&format!("fn_ret_map:{}", name))
                // #101: qualified-spelled calls consult exact markers only.
                || (is_simple_name(name)
                    && vars.contains_key(&format!("fn_ret_map:{}", bare)))
        }
        Expr::Map(_) => true,
        Expr::Index { array, index } => {
            if let Expr::String(field) = &**index {
                let key = format!("map_field_map:{}", field);
                if vars.contains_key(&key) {
                    return true;
                }
            }
            if let (Expr::Identifier(obj_name), Expr::String(field)) = (&**array, &**index) {
                let key = format!("map_map:{}.{}", obj_name, field);
                if vars.contains_key(&key) {
                    return true;
                }
            }
            false
        }
        Expr::FieldAccess { object, field } => {
            if let Expr::Identifier(obj_name) = &**object {
                let key = format!("{}.{}", obj_name, field);
                if let Some(var_type) = vars.get(&key) {
                    if matches!(var_type, VarType::Map(_)) {
                        return true;
                    }
                }
                if let Some(VarType::Struct { struct_name, .. }) = vars.get(obj_name) {
                    let field_key = format!("struct_field_map:{}.{}", struct_name, field);
                    if vars.contains_key(&field_key) {
                        return true;
                    }
                }
            }
            let global_field_key = format!("struct_field_map:{}", field);
            if !struct_field_kind_mixed(vars, field) && vars.contains_key(&global_field_key) {
                return true;
            }
            false
        }
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => {
            // Null-transparent AND (see is_string_expr): both non-null arms
            // must be maps for callers to assume a map outcome.
            (is_map_expr(then_branch, vars) || is_null_expr(then_branch, vars))
                && (is_map_expr(else_branch, vars) || is_null_expr(else_branch, vars))
                && (is_map_expr(then_branch, vars) || is_map_expr(else_branch, vars))
        }
        Expr::NullCoalesce { value, default } => {
            (is_map_expr(value, vars) || is_null_expr(value, vars))
                && (is_map_expr(default, vars) || is_null_expr(default, vars))
                && (is_map_expr(value, vars) || is_map_expr(default, vars))
        }
        Expr::OptionalFieldAccess { object, field } => is_map_expr(
            &Expr::FieldAccess {
                object: object.clone(),
                field: field.clone(),
            },
            vars,
        ),
        Expr::OptionalIndex { array, index } => is_map_expr(
            &Expr::Index {
                array: array.clone(),
                index: index.clone(),
            },
            vars,
        ),
        Expr::OptionalCall { callee, args } => is_map_expr(
            &Expr::Call {
                name: callee.clone(),
                args: args.clone(),
            },
            vars,
        ),
        Expr::ForceUnwrap(inner) => is_map_expr(inner, vars),
        _ => false,
    }
}

/// `is array` may fold to true only on exact evidence
/// (alya-lang/alya#70). Plain identifiers need the strict prologue
/// marker (`param_arr_strict:`): the may-marking (`VarType::Array`)
/// also covers merely-possible arrays (existential call-site rule,
/// alya-lang/alya#47), which folding must not trust. Transparent
/// shapes recurse; everything else keeps the legacy predicate.
pub fn is_array_fold_true(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Identifier(name) => vars.contains_key(&format!("param_arr_strict:{}", name)),
        Expr::ForceUnwrap(inner) => is_array_fold_true(inner, vars),
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => {
            (is_array_fold_true(then_branch, vars) || is_null_expr(then_branch, vars))
                && (is_array_fold_true(else_branch, vars) || is_null_expr(else_branch, vars))
                && (is_array_fold_true(then_branch, vars) || is_array_fold_true(else_branch, vars))
        }
        Expr::NullCoalesce { value, default } => {
            (is_array_fold_true(value, vars) || is_null_expr(value, vars))
                && (is_array_fold_true(default, vars) || is_null_expr(default, vars))
                && (is_array_fold_true(value, vars) || is_array_fold_true(default, vars))
        }
        _ => is_array_expr(expr, vars),
    }
}

/// `is string` may fold to true only on exact evidence
/// (alya-lang/alya#71). Plain identifiers need the strict prologue
/// marker (`param_str_strict:`): the may-marking also covers
/// merely-possible strings (existential per-call evidence gated only
/// by literal vetoes). Everything else keeps the legacy predicate.
pub fn is_string_fold_true(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Identifier(name) => vars.contains_key(&format!("param_str_strict:{}", name)),
        _ => is_string_expr(expr, vars),
    }
}

/// True when re-evaluating `expr` is side-effect free and
/// allocation-free, so a multi-branch cascade may evaluate it per
/// branch with identical semantics. Calls and freshly built values
/// (arrays, maps, structs, interpolated strings) evaluate once into a
/// hidden cell instead.
pub fn typeof_operand_is_repeatable(expr: &Expr) -> bool {
    match expr {
        Expr::Call { .. } | Expr::OptionalCall { .. } => false,
        Expr::Array(_) | Expr::Map(_) | Expr::StructInit { .. } | Expr::InterpolatedString(_) => {
            false
        }
        Expr::Binary { left, right, .. } => {
            typeof_operand_is_repeatable(left) && typeof_operand_is_repeatable(right)
        }
        Expr::Unary { expr, .. } | Expr::ForceUnwrap(expr) => typeof_operand_is_repeatable(expr),
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            typeof_operand_is_repeatable(array) && typeof_operand_is_repeatable(index)
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            typeof_operand_is_repeatable(object)
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            typeof_operand_is_repeatable(condition)
                && typeof_operand_is_repeatable(then_branch)
                && typeof_operand_is_repeatable(else_branch)
        }
        Expr::NullCoalesce { value, default } => {
            typeof_operand_is_repeatable(value) && typeof_operand_is_repeatable(default)
        }
        Expr::Cast { expr, .. } => typeof_operand_is_repeatable(expr),
        Expr::TypeCheck { expr, .. } => typeof_operand_is_repeatable(expr),
        _ => true,
    }
}

/// `is map` may fold to true only on exact evidence
/// (alya-lang/alya#70). Mirrors `is_array_fold_true`.
pub fn is_map_fold_true(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Identifier(name) => vars.contains_key(&format!("param_map_strict:{}", name)),
        Expr::ForceUnwrap(inner) => is_map_fold_true(inner, vars),
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => {
            (is_map_fold_true(then_branch, vars) || is_null_expr(then_branch, vars))
                && (is_map_fold_true(else_branch, vars) || is_null_expr(else_branch, vars))
                && (is_map_fold_true(then_branch, vars) || is_map_fold_true(else_branch, vars))
        }
        Expr::NullCoalesce { value, default } => {
            (is_map_fold_true(value, vars) || is_null_expr(value, vars))
                && (is_map_fold_true(default, vars) || is_null_expr(default, vars))
                && (is_map_fold_true(value, vars) || is_map_fold_true(default, vars))
        }
        _ => is_map_expr(expr, vars),
    }
}

/// Codegen-side twin of the inference sentinel check: true when global
/// field markers for `(sname, fname)` must not be emitted because mixed
/// literal kinds were observed program-wide. Reads then fall back to
/// runtime classification plus per-variable keys.
pub fn struct_field_markers_mixed_vars(
    vars: &HashMap<String, VarType>,
    sname: &str,
    fname: &str,
) -> bool {
    let bare_s = sname.rsplit("::").next().unwrap_or(sname);
    let bare_s = bare_s.rsplit("__").next().unwrap_or(bare_s);
    vars.contains_key(&format!("struct_field_mixed:{}.{}", sname, fname))
        || vars.contains_key(&format!("struct_field_mixed:{}.{}", bare_s, fname))
        || vars.contains_key(&format!("struct_field_mixed:{}", fname))
}

/// True when bare-global kind markers for `field` are unusable: mixed
/// literal kinds or conflicting declarations were observed, so no single
/// static kind serves every holder. Every bare-global fallback below
/// consults this (alya-lang/alya#131); qualified per-struct markers stay
/// precise and are unaffected.
fn struct_field_kind_mixed(vars: &HashMap<String, VarType>, field: &str) -> bool {
    vars.contains_key(&format!("struct_field_mixed:{}", field))
}

/// Post-seeding cleanup (alya-lang/alya#132): when one field name carries
/// markers from two or more kind families (string vs array vs map vs
/// float), no static kind serves every read, so every marker for that
/// field is unsound — including the qualified per-struct ones that typed
/// receivers consult directly.
///
/// The literal-kind sentinels (alya-lang/alya#113) and the annotation
/// conflicts (alya-lang/alya#131) both miss the case where every
/// construction site passes a *variable* (`payload: payload` in both the
/// string and the byte paths): each side observes DYN, no sentinel is
/// emitted, yet inference still records `struct_field_str:F.payload` on
/// the string side and `struct_field_arr:F.payload` on the array side.
/// Typed reads then take the losing static path (a byte array read back
/// as a C string truncates at the first NUL: `len` reports 1 for
/// `[72, 0]`, element loads return header garbage, a later `push`
/// segfaults).
///
/// Contradictory keys are removed outright (not just gated) so readers
/// that never consult the sentinel cannot pick the losing path either;
/// a `struct_field_mixed:{field}` sentinel is left behind so write sites
/// and bare-global fallbacks demote the field to dynamic dispatch.
/// Fields with markers from a single family are untouched.
pub fn suppress_contradictory_struct_field_markers(vars: &mut HashMap<String, VarType>) {
    const F_STR: u8 = 1;
    const F_ARR: u8 = 2;
    const F_MAP: u8 = 4;
    const F_FLT: u8 = 8;

    fn family_of(key: &str) -> Option<(u8, &str)> {
        // Longest (sub-kind) prefixes first: `struct_field_arr_str:` etc.
        // refine array elements but are still array-kind markers.
        for (prefix, fam) in [
            ("struct_field_arr_str:", F_ARR),
            ("struct_field_arr_flt:", F_ARR),
            ("struct_field_arr_int:", F_ARR),
            ("struct_field_arr:", F_ARR),
            ("struct_field_str:", F_STR),
            ("struct_field_map:", F_MAP),
            ("struct_field_flt:", F_FLT),
        ] {
            if let Some(rest) = key.strip_prefix(prefix) {
                return Some((fam, rest));
            }
        }
        None
    }

    fn bare_struct(s: &str) -> &str {
        let b = s.rsplit("::").next().unwrap_or(s);
        b.rsplit("__").next().unwrap_or(b)
    }

    // field -> kind mask (bare-global markers).
    let mut bare_fams: HashMap<String, u8> = HashMap::new();
    // (bare struct, field) -> kind mask (qualified markers).
    let mut qual_fams: HashMap<(String, String), u8> = HashMap::new();
    // (bare struct, field) -> qualified keys in that group.
    let mut qual_keys: HashMap<(String, String), Vec<String>> = HashMap::new();
    // field -> bare-global keys.
    let mut bare_keys: HashMap<String, Vec<String>> = HashMap::new();

    for key in vars.keys() {
        let Some((fam, rest)) = family_of(key) else {
            continue;
        };
        if let Some((s, f)) = rest.rsplit_once('.') {
            let group = (bare_struct(s).to_string(), f.to_string());
            *qual_fams.entry(group.clone()).or_default() |= fam;
            qual_keys.entry(group).or_default().push(key.clone());
        } else {
            *bare_fams.entry(rest.to_string()).or_default() |= fam;
            bare_keys
                .entry(rest.to_string())
                .or_default()
                .push(key.clone());
        }
    }

    let mut remove: Vec<String> = Vec::new();
    let mut mixed_fields: HashSet<String> = HashSet::new();
    for (field, mask) in &bare_fams {
        if mask.count_ones() > 1 {
            mixed_fields.insert(field.clone());
            remove.extend(bare_keys[field].iter().cloned());
        }
    }
    for (group, mask) in &qual_fams {
        if mask.count_ones() > 1 {
            mixed_fields.insert(group.1.clone());
            remove.extend(qual_keys[group].iter().cloned());
        }
    }

    for key in remove {
        vars.remove(&key);
    }
    for field in mixed_fields {
        vars.insert(format!("struct_field_mixed:{}", field), VarType::Number(0));
    }
}

/// True when an `Index` read routes through `fn_get` (map path) rather
/// than a direct array load. Mirrors the routing condition in the
/// expression codegen: map-typed base, string-typed key, or string
/// literal key. Only `fn_get` returns an entry kind tag (x64: %edx,
/// arm64: w1); direct array loads leave that register holding the
/// index/length instead, so tag dispatch must be gated on this.
pub fn is_map_read_index(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    if let Expr::Index { array, index } = expr {
        is_map_expr(array, vars)
            || is_string_expr(index, vars)
            || matches!(**index, Expr::String(_))
    } else {
        false
    }
}

/// True when an `Index` read is guaranteed to deliver an array-slot
/// kind tag on x64 (Phase 1, alya-lang/alya#39): the base is an
/// identifier known to hold an array. `VarType::Array` excludes
/// struct/string/map bindings, and map-routed reads are excluded
/// (they carry entry tags through `fn_get` instead). Mirrors the
/// expression-codegen routing: struct `operator[]` rewrites and map
/// `get` calls never reach the array loader. Field-access bases stay
/// on the legacy path for now.
pub fn is_array_kind_read(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    if let Expr::Index { array, .. } = expr {
        if is_map_read_index(expr, vars) {
            return false;
        }
        if let Expr::Identifier(name) = &**array {
            return matches!(vars.get(name), Some(VarType::Array(_)));
        }
    }
    false
}

/// Binary ops where a float operand is a strict error unless statically
/// routed (alya-lang/alya#39 Phase 2b): arithmetic, bitwise, and
/// comparisons. Logical And/Or keep legacy truthiness.
pub fn is_strict_dynamic_op(op: &BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Modulo
            | BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
    )
}

/// True when an expression delivers a trustworthy value-kind tag in
/// the tag register (x64: `%edx`, arm64: `w1`): map-routed reads via
/// `fn_get`, plain array-identifier reads via the slot sidecar,
/// dynamically-typed element reads (Phase 2b), non-float ternaries with
/// a tag-carrying arm (codegen materializes every other arm), or calls
/// into return-tagged functions (every return leaves `(value, tag)`).
/// Mirrors the expression-codegen routing exactly; struct `operator[]`
/// rewrites and string-typed bases never reach a tag-producing loader.
pub fn is_tag_carrying_read(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    if let Expr::Ternary {
        then_branch,
        else_branch,
        ..
    } = expr
    {
        return !is_float_expr(expr, vars)
            && (ternary_arm_carries(then_branch, vars) || ternary_arm_carries(else_branch, vars));
    }
    if let Expr::Call { name, .. } = expr {
        // Return-tag protocol (Phase 2b, #39): the callee guarantees
        // (value, tag) on every return path. Unqualified callees
        // (recursion, dynamics) keep legacy behavior; forward
        // references are ordered callee-first (#55-C). Bare-name
        // lookup mirrors fn_ret_flt — but only for simple spellings
        // (#101): a qualified-spelled call must never inherit markers
        // set by an unrelated same-bare function.
        if vars.contains_key(&format!("fn_ret_tagged:{}", name)) {
            return true;
        }
        if !is_simple_name(name) {
            return false;
        }
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        return vars.contains_key(&format!("fn_ret_tagged:{}", bare));
    }
    is_map_read_index(expr, vars)
        || is_array_kind_read(expr, vars)
        || is_dynamic_element_read(expr, vars)
}

/// True when a ternary arm delivers its tag without materialization:
/// tag-carrying Index reads, or nested ternaries that materialize their
/// own (recursion terminates: arms are strictly smaller exprs).
/// Codegen consults the same rule, so predicate and emission agree.
pub fn ternary_arm_carries(arm: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match arm {
        Expr::Index { .. } => {
            is_map_read_index(arm, vars)
                || is_array_kind_read(arm, vars)
                || is_dynamic_element_read(arm, vars)
        }
        Expr::Ternary { .. } => is_tag_carrying_read(arm, vars),
        _ => false,
    }
}

/// True when an `Index` read has statically-unknown element kind
/// (alya-lang/alya#39 Phase 2b): an array-typed base without a proven
/// whole-array string/float claim, or an untyped param base (params take
/// the array path with sidecar tags). Proven string/float arrays keep
/// their exact paths; proven-int arrays agree with the tag anyway.
/// Consumers may trust the slot/entry tag in the tag register.
/// Boundary: a non-indexable dynamic passed in crashes identically with
/// or without tag dispatch (layout is assumed either way).
pub fn is_dynamic_element_read(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    if let Expr::Index { array, .. } = expr {
        if is_map_read_index(expr, vars) {
            return false;
        }
        // String-routed bases (char_at) leave no slot tag behind, so
        // consumers must keep their static paths for them.
        if is_string_expr(array, vars) {
            return false;
        }
        if let Expr::Identifier(name) = &**array {
            if vars.contains_key(&format!("param_is_untyped:{}", name)) {
                return true;
            }
            if matches!(vars.get(name), Some(VarType::Array(_))) {
                let proven_str = vars.contains_key(&format!("arr_is_str:{}", name))
                    && !vars.contains_key(&format!("arr_nonstr:{}", name));
                let proven_flt = vars.contains_key(&format!("arr_is_flt:{}", name))
                    && !vars.contains_key(&format!("arr_nonflt:{}", name));
                return !proven_str && !proven_flt;
            }
            // #106: unknown/dynamic locals flow into the same array
            // fallback, which loads the slot tag anyway, so consumers
            // can dispatch on it. Proven scalars, structs, maps and
            // friends keep their static paths; a non-indexable dynamic
            // faults at the bounds check either way (layout is assumed
            // by the fallback itself).
            if matches!(vars.get(name), None | Some(VarType::Number(_)))
                && !vars.contains_key(&format!("var_is_int:{}", name))
            {
                return true;
            }
            return false;
        }
        // Non-identifier bases (calls, nested reads, field paths) take
        // the same array fallback with its slot-tag load (#106), unless
        // string-routed above. Struct operator[] rewrites carry the
        // callee's own tag, which dispatch handles correctly too.
        return true;
    }
    false
}

pub fn is_string_array(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Array(elems) => !elems.is_empty() && elems.iter().all(|e| is_string_expr(e, vars)),
        Expr::Identifier(name) => {
            vars.contains_key(&format!("arr_is_str:{}", name))
                && !vars.contains_key(&format!("arr_nonstr:{}", name))
        }
        Expr::FieldAccess { object, field } => {
            if let Expr::Identifier(obj_name) = &**object {
                if let Some(VarType::Struct { struct_name, .. }) = vars.get(obj_name) {
                    let bare = struct_name.rsplit("::").next().unwrap_or(struct_name);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    let field_key = format!("struct_field_arr_str:{}.{}", struct_name, field);
                    let bare_key = format!("struct_field_arr_str:{}.{}", bare, field);
                    if vars.contains_key(&field_key) || vars.contains_key(&bare_key) {
                        return true;
                    }
                }
            }
            let global_field_key = format!("struct_field_arr_str:{}", field);
            !struct_field_kind_mixed(vars, field) && vars.contains_key(&global_field_key)
        }
        Expr::Call { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            matches!(
                bare,
                // NOTE: `split` is deliberately absent: `Tensor.split`
                // (and any user `split`) returns non-string arrays, so a
                // static string-element claim miscompiles reads/stores.
                "args"
                    | "cli_args"
                    | "lines"
                    | "read_lines"
                    | "keys"
                    | "list_dir"
                    | "read_dir"
                    | "fs_list_dir"
                    | "fs_read_dir"
                    | "list_dir_recursive"
                    | "glob"
                    | "glob_dir"
                    | "glob_filter"
            ) || vars.contains_key(&format!("fn_ret_str_arr:{}", name))
                || vars.contains_key(&format!("fn_ret_str_arr:{}", bare))
        }
        Expr::ForceUnwrap(inner) => is_string_array(inner, vars),
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => {
            // Null-transparent AND (see is_string_expr): both non-null arms
            // must be string arrays for callers to assume string elements.
            (is_string_array(then_branch, vars) || is_null_expr(then_branch, vars))
                && (is_string_array(else_branch, vars) || is_null_expr(else_branch, vars))
                && (is_string_array(then_branch, vars) || is_string_array(else_branch, vars))
        }
        Expr::NullCoalesce { value, default } => {
            (is_string_array(value, vars) || is_null_expr(value, vars))
                && (is_string_array(default, vars) || is_null_expr(default, vars))
                && (is_string_array(value, vars) || is_string_array(default, vars))
        }
        _ => false,
    }
}

pub fn is_float_array(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Array(elems) => {
            // Mixed literals are NOT float arrays (#95): every element
            // must be float, mirroring is_string_array's `all` below.
            // First-only marking misread `[1.5, 1][1]` as f64 bits.
            !elems.is_empty() && elems.iter().all(|e| is_float_expr(e, vars))
        }
        Expr::Identifier(name) => {
            vars.contains_key(&format!("arr_is_flt:{}", name))
                && !vars.contains_key(&format!("arr_nonflt:{}", name))
        }
        Expr::FieldAccess { object, field } => {
            if let Expr::Identifier(obj_name) = &**object {
                if let Some(VarType::Struct { struct_name, .. }) = vars.get(obj_name) {
                    let bare = struct_name.rsplit("::").next().unwrap_or(struct_name);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    let field_key = format!("struct_field_arr_flt:{}.{}", struct_name, field);
                    let bare_key = format!("struct_field_arr_flt:{}.{}", bare, field);
                    if vars.contains_key(&field_key) || vars.contains_key(&bare_key) {
                        return true;
                    }
                }
            }
            let global_field_key = format!("struct_field_arr_flt:{}", field);
            !struct_field_kind_mixed(vars, field) && vars.contains_key(&global_field_key)
        }
        Expr::ForceUnwrap(inner) => is_float_array(inner, vars),
        _ => false,
    }
}

pub fn is_float_expr(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Float(_) => true,
        Expr::Number(_) => false,
        Expr::Identifier(name) => {
            matches!(vars.get(name), Some(VarType::Float(_)))
        }
        Expr::FieldAccess { object, field } => {
            if let Expr::Identifier(obj_name) = &**object {
                let key = format!("{}.{}", obj_name, field);
                if matches!(vars.get(&key), Some(VarType::Float(_))) {
                    return true;
                }
                if let Some(VarType::Struct { struct_name, .. }) = vars.get(obj_name) {
                    let field_key = format!("struct_field_flt:{}.{}", struct_name, field);
                    if vars.contains_key(&field_key) {
                        return true;
                    }
                }
            }
            let global_field_key = format!("struct_field_flt:{}", field);
            if !struct_field_kind_mixed(vars, field) && vars.contains_key(&global_field_key) {
                return true;
            }
            false
        }
        Expr::Binary {
            left,
            op:
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Modulo,
            right,
        } => is_float_expr(left, vars) || is_float_expr(right, vars),
        Expr::Unary {
            op: UnaryOp::Negate,
            expr,
        } => is_float_expr(expr, vars),
        Expr::Index { array, index } => {
            if let (Expr::Identifier(arr_name), Expr::Number(idx)) = (&**array, &**index) {
                let key = format!("tuple_elem_flt:{}:{}", arr_name, *idx as usize);
                if vars.contains_key(&key) {
                    return true;
                }
            }
            if let Expr::String(field) = &**index {
                let key = format!("map_field_flt:{}", field);
                if vars.contains_key(&key) {
                    return true;
                }
            }
            if let (Expr::Identifier(obj_name), Expr::String(field)) = (&**array, &**index) {
                let key = format!("map_flt:{}.{}", obj_name, field);
                if vars.contains_key(&key) {
                    return true;
                }
            }
            is_float_array(array, vars)
        }
        Expr::Call { name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if bare == "get" && args.len() >= 3 && is_float_expr(&args[2], vars) {
                return true;
            }
            // Receiver-qualified override: method names like `min` or
            // `sum_horizontal` are shared between float vectors (f32x8,
            // f64x4) and integer vectors (i32x8, i64x4), so the bare
            // `fn_ret_flt:{method}` marker cannot decide them. When the
            // receiver resolves to a struct, the qualified markers are
            // authoritative: an exact integer-marker match rules out
            // float even if a bare float marker exists (and vice versa).
            // Qualified integer evidence is precise (never recorded
            // bare), so it wins ties.
            if let Some(first) = args.first() {
                if let Some(sname) = receiver_struct_name(first, vars) {
                    let bare_s = sname.rsplit("::").next().unwrap_or(&sname);
                    let bare_s = bare_s.rsplit("__").next().unwrap_or(bare_s);
                    for ss in [sname.as_str(), bare_s] {
                        for cc in [name.as_str(), bare] {
                            if vars.contains_key(&format!("fn_ret_int:{}__{}", ss, cc))
                                || vars.contains_key(&format!("fn_ret_int:{}::{}", ss, cc))
                            {
                                return false;
                            }
                        }
                    }
                    // Suffix scan mirrors the string arm below: method
                    // markers may live under an import-alias prefix
                    // (`m::Box__sum`) while the receiver resolves
                    // unaliased (`Box`). The struct-qualified suffix
                    // keeps it precise to this struct+method pair.
                    for ss in [sname.as_str(), bare_s] {
                        for cc in [name.as_str(), bare] {
                            let suffix_us = format!("__{}__{}", ss, cc);
                            let suffix_col = format!("::{}__{}", ss, cc);
                            if vars.keys().any(|k| {
                                k.starts_with("fn_ret_int:")
                                    && (k.ends_with(&suffix_us) || k.ends_with(&suffix_col))
                            }) {
                                return false;
                            }
                        }
                    }
                    for ss in [sname.as_str(), bare_s] {
                        for cc in [name.as_str(), bare] {
                            if vars.contains_key(&format!("fn_ret_flt:{}__{}", ss, cc))
                                || vars.contains_key(&format!("fn_ret_flt:{}::{}", ss, cc))
                            {
                                return true;
                            }
                        }
                    }
                    for ss in [sname.as_str(), bare_s] {
                        for cc in [name.as_str(), bare] {
                            let suffix_us = format!("__{}__{}", ss, cc);
                            let suffix_col = format!("::{}__{}", ss, cc);
                            if vars.keys().any(|k| {
                                k.starts_with("fn_ret_flt:")
                                    && (k.ends_with(&suffix_us) || k.ends_with(&suffix_col))
                            }) {
                                return true;
                            }
                        }
                    }
                }
            }
            matches!(
                bare,
                "float"
                    | "to_float"
                    | "parse_float"
                    | "sin"
                    | "cos"
                    | "tan"
                    | "mean"
                    | "deg_to_rad"
                    | "rad_to_deg"
                    | "radians"
                    | "degrees"
                    | "lerp"
                    | "norm"
                    | "smoothstep"
                    | "variance"
                    | "rand_float"
                    | "rand_float_range"
                    | "float_range"
                    | "rand_rng_float"
                    | "uniform_float"
                    | "rng_float"
                    | "dist_float"
                    | "dist_uniform_float"
                    | "dist_normal"
                    | "dist_exponential"
                    | "normal"
                    | "exponential"
                    | "native_sin"
                    | "native_cos"
                    | "native_tan"
                    | "native_asin"
                    | "native_acos"
                    | "native_atan"
                    | "native_atan2"
                    | "native_sinh"
                    | "native_cosh"
                    | "native_tanh"
                    | "native_asinh"
                    | "native_acosh"
                    | "native_atanh"
                    | "native_log"
                    | "native_log2"
                    | "native_log10"
                    | "native_log1p"
                    | "native_exp"
                    | "native_exp2"
                    | "native_expm1"
                    | "native_sqrt"
                    | "native_cbrt"
                    | "native_erf"
                    | "native_erfc"
                    | "native_lgamma"
                    | "native_tgamma"
                    | "native_ceil"
                    | "native_floor"
                    | "native_fmod"
                    | "native_pow"
                    | "native_copysign"
                    | "native_nextafter"
                    | "native_fma"
                    | "asin"
                    | "acos"
                    | "atan"
                    | "atan2"
                    | "sinh"
                    | "cosh"
                    | "tanh"
                    | "asinh"
                    | "acosh"
                    | "atanh"
                    | "log_n"
                    | "log2"
                    | "log10"
                    | "log1p"
                    | "exp_f"
                    | "exp2"
                    | "expm1"
                    | "fmod"
                    | "sqrt_f"
                    | "cbrt"
                    | "pow_f"
                    | "powi"
                    | "erf"
                    | "erfc"
                    | "lgamma"
                    | "tgamma"
                    | "copysign"
                    | "nextafter"
                    | "fma"
                    | "fract"
                    | "recip"
            ) || vars.contains_key(&format!("fn_ret_flt:{}", name))
                // #101: a qualified-spelled call must never inherit
                // markers set by an unrelated same-bare function.
                || (is_simple_name(name)
                    && vars.contains_key(&format!("fn_ret_flt:{}", bare)))
        }
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => is_float_expr(then_branch, vars) || is_float_expr(else_branch, vars),
        Expr::NullCoalesce { value, default } => {
            is_float_expr(value, vars) || is_float_expr(default, vars)
        }
        Expr::OptionalFieldAccess { object, field } => is_float_expr(
            &Expr::FieldAccess {
                object: object.clone(),
                field: field.clone(),
            },
            vars,
        ),
        Expr::OptionalIndex { array, index } => is_float_expr(
            &Expr::Index {
                array: array.clone(),
                index: index.clone(),
            },
            vars,
        ),
        Expr::OptionalCall { callee, args } => is_float_expr(
            &Expr::Call {
                name: callee.clone(),
                args: args.clone(),
            },
            vars,
        ),
        Expr::Cast {
            expr: inner,
            target,
        } => {
            let t = target.to_lowercase();
            if t == "float" || t == "f64" || t == "f32" {
                true
            } else if t == "int"
                || t == "i64"
                || t == "i32"
                || t == "usize"
                || t == "str"
                || t == "string"
            {
                false
            } else {
                is_float_expr(inner, vars)
            }
        }
        Expr::ForceUnwrap(inner) => is_float_expr(inner, vars),
        _ => false,
    }
}

pub fn is_null_expr(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Null => true,
        Expr::Identifier(name) => matches!(vars.get(name), Some(VarType::Null(_))),
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => is_null_expr(then_branch, vars) && is_null_expr(else_branch, vars),
        Expr::NullCoalesce { value, default } => {
            is_null_expr(value, vars) && is_null_expr(default, vars)
        }
        // Force unwrap asserts non-null (traps on null at runtime).
        Expr::ForceUnwrap(_) => false,
        _ => false,
    }
}

/// Assigned-kind classes for the nullable-heap proof below.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AssignedKind {
    /// `null` literal: refcount guards skip it.
    Null,
    /// Heap-constructor literal (array/map/struct) or a call proven to
    /// return heap (struct constructor, `fn_ret_arr/map/struct`
    /// markers — the same evidence `is_heap_expression` trusts):
    /// a fresh live heap object.
    Heap,
    /// Anything else (calls, identifiers, reads, arithmetic...):
    /// unknown, never proven.
    Other,
}

fn classify_assigned(
    expr: &Expr,
    vars: &HashMap<String, VarType>,
    structs: &HashMap<String, crate::codegen::context::StructDefInfo>,
) -> AssignedKind {
    match expr {
        Expr::Null => AssignedKind::Null,
        Expr::Array(_) | Expr::Map(_) | Expr::StructInit { .. } => AssignedKind::Heap,
        Expr::Call { name, .. } => {
            // Exactly the evidence `is_heap_expression` trusts for calls
            // (struct constructors, `fn_ret_struct` markers, array/map
            // calls) — no `fn_ret_fresh`: freshness is not heapness.
            if structs.contains_key(name)
                || matches!(
                    vars.get(&format!("fn_ret_struct:{}", name)),
                    Some(VarType::Struct { .. })
                )
                || is_array_expr(expr, vars)
                || is_map_expr(expr, vars)
            {
                AssignedKind::Heap
            } else {
                AssignedKind::Other
            }
        }
        _ => AssignedKind::Other,
    }
}

/// Nullable-heap locals for the probe-free refcount fast path.
///
/// Walks a function body and returns the locals that can only ever hold
/// `null` or heap values: every syntactic assignment to them (including
/// inside nested blocks and closures, which may capture and write outer
/// locals) is a `null` literal, a heap-constructor literal, or a call
/// proven to return heap (struct constructors, `fn_ret_arr/map/struct`
/// markers — the same evidence `is_heap_expression` trusts), with at
/// least one heap assignment. Reads of such locals take the direct
/// (probe-free) retain/release: `null` skips via the guards, heap
/// objects via the magic check. Anything else — unlisted statement
/// forms, loop/catch variables (iterable/throwable values are unknown),
/// parameters (callers assign invisibly), globals (assigned anywhere in
/// the program), shadowed names (several `let` bindings), any other
/// right-hand side — stays unproven and keeps the probed call, so the
/// analysis only ever removes syscalls, never safety.
///
/// Soundness rests on syntactic completeness: every assignment to a
/// local in the language is a `let`, `const`, `=`-assign, or loop/catch
/// binding, all covered above; there are no references, eval, or other
/// invisible writers to locals.
pub fn nullable_heap_locals(
    body: &[&Stmt],
    params: &[String],
    globals: &std::collections::HashSet<String>,
    vars: &HashMap<String, VarType>,
    structs: &HashMap<String, crate::codegen::context::StructDefInfo>,
) -> HashSet<String> {
    use std::collections::HashMap as Map;
    let mut lets: Map<String, usize> = Map::new();
    let mut assigns: Map<String, Vec<AssignedKind>> = Map::new();
    let mut disqualified: HashSet<String> = params.iter().cloned().collect();

    fn walk_refs(
        body: &[Stmt],
        lets: &mut Map<String, usize>,
        assigns: &mut Map<String, Vec<AssignedKind>>,
        disqualified: &mut HashSet<String>,
        vars: &HashMap<String, VarType>,
        structs: &HashMap<String, crate::codegen::context::StructDefInfo>,
    ) {
        let refs: Vec<&Stmt> = body.iter().collect();
        walk(&refs, lets, assigns, disqualified, vars, structs);
    }

    fn walk(
        stmts: &[&Stmt],
        lets: &mut Map<String, usize>,
        assigns: &mut Map<String, Vec<AssignedKind>>,
        disqualified: &mut HashSet<String>,
        vars: &HashMap<String, VarType>,
        structs: &HashMap<String, crate::codegen::context::StructDefInfo>,
    ) {
        for stmt in stmts {
            match *stmt {
                Stmt::Let { name, value, .. } => {
                    *lets.entry(name.clone()).or_insert(0) += 1;
                    assigns
                        .entry(name.clone())
                        .or_default()
                        .push(classify_assigned(value, vars, structs));
                }
                Stmt::Const { name, value } => {
                    *lets.entry(name.clone()).or_insert(0) += 1;
                    assigns
                        .entry(name.clone())
                        .or_default()
                        .push(classify_assigned(value, vars, structs));
                }
                Stmt::Assign { name, value } => {
                    assigns
                        .entry(name.clone())
                        .or_default()
                        .push(classify_assigned(value, vars, structs));
                }
                Stmt::For { var, body, .. } => {
                    disqualified.insert(var.clone());
                    walk_refs(body, lets, assigns, disqualified, vars, structs);
                }
                Stmt::ForEach {
                    var,
                    value_var,
                    body,
                    ..
                } => {
                    disqualified.insert(var.clone());
                    if let Some(vv) = value_var {
                        disqualified.insert(vv.clone());
                    }
                    walk_refs(body, lets, assigns, disqualified, vars, structs);
                }
                Stmt::Function {
                    name, params, body, ..
                } => {
                    // Nested closures may capture and write outer locals:
                    // their params are invisible to us, their bodies are not.
                    // The function name itself could also shadow a variable.
                    disqualified.insert(name.clone());
                    for p in params {
                        disqualified.insert(p.clone());
                    }
                    walk_refs(body, lets, assigns, disqualified, vars, structs);
                }
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    walk_refs(then_block, lets, assigns, disqualified, vars, structs);
                    if let Some(eb) = else_block {
                        walk_refs(eb, lets, assigns, disqualified, vars, structs);
                    }
                }
                Stmt::While { body, .. } | Stmt::Repeat { body } => {
                    walk_refs(body, lets, assigns, disqualified, vars, structs);
                }
                Stmt::TryCatch {
                    try_block,
                    catch_var,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    if let Some(cv) = catch_var {
                        disqualified.insert(cv.clone());
                    }
                    walk_refs(try_block, lets, assigns, disqualified, vars, structs);
                    walk_refs(catch_block, lets, assigns, disqualified, vars, structs);
                    if let Some(fb) = finally_block {
                        walk_refs(fb, lets, assigns, disqualified, vars, structs);
                    }
                }
                Stmt::Defer(inner) | Stmt::Pub(inner) => {
                    walk(
                        std::slice::from_ref(&inner.as_ref()),
                        lets,
                        assigns,
                        disqualified,
                        vars,
                        structs,
                    );
                }
                // No variable assignments: imports, definitions, externs,
                // expression statements (expressions hold no statements),
                // control transfers, and container-slot writes
                // (`IndexAssign`/`FieldAssign` target slots, not locals).
                Stmt::Import { .. }
                | Stmt::ExternBlock { .. }
                | Stmt::Say(_)
                | Stmt::StructDef { .. }
                | Stmt::EnumDef { .. }
                | Stmt::Return(_)
                | Stmt::Break
                | Stmt::Continue
                | Stmt::Expr(_)
                | Stmt::Throw(_)
                | Stmt::IndexAssign { .. }
                | Stmt::FieldAssign { .. }
                | Stmt::InterfaceDef { .. } => {}
            }
        }
    }

    walk(
        body,
        &mut lets,
        &mut assigns,
        &mut disqualified,
        vars,
        structs,
    );

    assigns
        .into_iter()
        .filter(|(name, kinds)| {
            !disqualified.contains(name)
                && !globals.contains(name)
                && lets.get(name).copied().unwrap_or(0) <= 1
                && !kinds.is_empty()
                && kinds.iter().all(|k| *k != AssignedKind::Other)
                && kinds.contains(&AssignedKind::Heap)
        })
        .map(|(name, _)| name)
        .collect()
}

pub fn is_number_expr(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Number(_) => true,
        Expr::Identifier(name) => {
            !vars.contains_key(&format!("param_is_untyped:{}", name))
                && matches!(vars.get(name), Some(VarType::Number(_)))
        }
        Expr::Binary { left, op, right } => match op {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Modulo
            | BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr => {
                !is_string_expr(left, vars)
                    && !is_string_expr(right, vars)
                    && !is_float_expr(left, vars)
                    && !is_float_expr(right, vars)
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
            | BinaryOp::And
            | BinaryOp::Or
            | BinaryOp::In
            | BinaryOp::NotIn
            | BinaryOp::Range
            | BinaryOp::RangeInclusive => true,
        },
        Expr::Unary { op, expr } => match op {
            UnaryOp::Negate | UnaryOp::BitNot => !is_float_expr(expr, vars),
            UnaryOp::Not => true,
        },
        Expr::Call { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            matches!(
                bare,
                "len" | "arr_len" | "ord" | "time" | "clock_ms" | "rand" | "rand_int" | "int"
                    | "to_int"
                    | "parse_int"
                    // File/FS status builtins return int flags (0/1), never
                    // heap pointers.
                    | "file_exists"
                    | "write_file"
                    | "append_file"
                    | "write_bytes"
                    | "append_bytes"
                    | "delete_file"
                    | "remove_file"
                // Bitwise builtins are int->int word ops (logical shifts);
                // results are machine words, never heap pointers, so stores
                // must skip rc_retain (it faults on large 8-aligned ints).
                | "bit_and" | "bit_or" | "bit_xor" | "bit_shl" | "bit_shr" | "bit_not"
            )
            // Functions with explicit `-> int` annotations are recorded as
            // `fn_ret_int:` (enforced by the type checker), so their results
            // are exact int facts too (e.g. crypto `_u32_le` words pushed
            // into state arrays must not retain).
            || call_returns_known_int(name, vars)
        }
        Expr::ForceUnwrap(inner) => is_number_expr(inner, vars),
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => is_number_expr(then_branch, vars) && is_number_expr(else_branch, vars),
        Expr::NullCoalesce { value, default } => {
            is_number_expr(value, vars) && is_number_expr(default, vars)
        }
        _ => false,
    }
}

/// True for explicit integer-scalar annotations (`int`, `i64`, ...).
pub fn is_int_scalar_annotation(ann: &str) -> bool {
    let t = ann.trim();
    let base = t.rsplit("::").next().unwrap_or(t);
    let base = base.rsplit("__").next().unwrap_or(base);
    matches!(
        base,
        "int"
            | "i64"
            | "isize"
            | "uint"
            | "u64"
            | "usize"
            | "i32"
            | "i16"
            | "i8"
            | "u32"
            | "u16"
            | "u8"
            | "byte"
            | "bool"
            | "boolean"
            | "char"
            | "rune"
    )
}

/// True for explicit integer-array annotations (`int[]`, `i64[]`, ...).
/// Explicit collection annotations are enforced by the type checker, so
/// they are exact: every element is a machine word, never a heap pointer.
/// Used to skip `rc_retain` on element reads stored into collections
/// (retaining a large 8-aligned int faults inside `rc_retain`).
pub fn is_int_array_annotation(ann: &str) -> bool {
    let t = ann.trim();
    if !t.ends_with("[]") {
        return false;
    }
    is_int_scalar_annotation(&t[..t.len() - 2])
}

/// Returns true when `expr` is provably NOT a plain number (a string,
/// array, map, or statically-known struct value).
///
/// Used to guard int<->float bit conversions: feeding a pointer (string,
/// array, map, struct) into `cvtsi2sdq`/`cvttsd2siq` silently produces
/// garbage. Unknown (dynamic) expressions return false so that existing
/// behavior for them is preserved exactly.
pub fn is_definitely_not_numeric(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    if is_string_expr(expr, vars) || is_array_expr(expr, vars) || is_map_expr(expr, vars) {
        return true;
    }
    if let Expr::Identifier(name) = expr {
        if matches!(vars.get(name), Some(VarType::Struct { .. })) {
            return true;
        }
    }
    false
}

/// Static value-kind tag for element stores (alya-lang/alya#39 Phase 1).
/// Precedence mirrors the `arr_is_str` marking on the push path: string
/// first, then float, then array/map, then int. Struct values report
/// through their variable type. Unknowns return KIND_UNKNOWN so readers
/// fall back to inference.
pub fn value_kind_tag(expr: &Expr, vars: &HashMap<String, VarType>) -> i64 {
    if let Some(kind) = kind_of_literal(expr) {
        return kind;
    }
    if is_string_expr(expr, vars) {
        return KIND_STRING;
    }
    if is_float_expr(expr, vars) {
        return KIND_FLOAT;
    }
    if is_array_expr(expr, vars) {
        return KIND_ARRAY;
    }
    if is_map_expr(expr, vars) {
        return KIND_MAP;
    }
    // NOTE: `is_number_expr` is true for ANY Number-typed local, including
    // dynamic values (e.g. `let doc = yaml_parse(part)` holding a map at
    // runtime). Storing KIND_INT for those corrupts readers (map dispatched
    // as int faults). Only a proven-int identifier (`var_is_int`) is an
    // exact INT fact; other shapes keep the sound `is_number_expr` rule
    // (literals, arithmetic, bit/int builtins).
    let is_exact_int = match expr {
        Expr::Identifier(name) => vars.contains_key(&format!("var_is_int:{}", name)),
        _ => is_number_expr(expr, vars),
    };
    if is_exact_int {
        return KIND_INT;
    }
    if let Expr::Identifier(name) = expr {
        if matches!(vars.get(name), Some(VarType::Struct { .. })) {
            return KIND_STRUCT;
        }
    }
    if let Expr::Call { name, .. } = expr {
        // Struct-returning calls (explicit `-> Struct` annotations seed
        // `fn_ret_struct`, like `fn_ret_arr` feeds is_array_expr above):
        // without this their slots store UNKNOWN and container-free
        // cascades cannot see them (alya-lang/alya#79).
        if call_returns_struct(name, vars) {
            return KIND_STRUCT;
        }
    }
    KIND_UNKNOWN
}

/// True when the callee is known to return a struct (explicit annotation
/// or inference marker), checking the same name spellings as the array
/// markers (`name`, `bare`, `__`/`::` swaps).
pub fn call_returns_struct(name: &str, vars: &HashMap<String, VarType>) -> bool {
    let bare = name.rsplit("::").next().unwrap_or(name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    vars.contains_key(&format!("fn_ret_struct:{}", name))
        // #101: qualified-spelled calls consult exact markers only.
        || (is_simple_name(name)
            && vars.contains_key(&format!("fn_ret_struct:{}", bare)))
        || vars.contains_key(&format!("fn_ret_struct:{}", name.replace("::", "__")))
        || vars.contains_key(&format!("fn_ret_struct:{}", name.replace("__", "::")))
}

/// True when the callee provably returns a freshly-owned value on every
/// path (literals, direct struct constructors, scalars, null): the
/// caller receives a dedicated reference it may release
/// (alya-lang/alya#79). Anything else (variables, reads, general calls,
/// unknown shapes) may alias caller-visible state.
pub fn call_returns_fresh_value(name: &str, vars: &HashMap<String, VarType>) -> bool {
    let bare = name.rsplit("::").next().unwrap_or(name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    vars.contains_key(&format!("fn_ret_fresh:{}", name))
        // #101: qualified-spelled calls consult exact markers only (a
        // stale freshness claim is use-after-free, alya-lang/alya#79).
        || (is_simple_name(name)
            && vars.contains_key(&format!("fn_ret_fresh:{}", bare)))
        || vars.contains_key(&format!("fn_ret_fresh:{}", name.replace("::", "__")))
        || vars.contains_key(&format!("fn_ret_fresh:{}", name.replace("__", "::")))
}

/// Worker for `fn_returns_fresh_value`: every `return` in the body must
/// yield a fresh-owned expression. Propagates freshness across known fresh
/// functions and transparent operators (alya-lang/alya#81).
pub fn fn_returns_fresh_value_ext(
    body: &[Stmt],
    struct_names: &std::collections::HashSet<String>,
    fresh_fns: &std::collections::HashSet<String>,
) -> bool {
    fn fresh_expr(
        e: &Expr,
        struct_names: &std::collections::HashSet<String>,
        fresh_fns: &std::collections::HashSet<String>,
    ) -> bool {
        match e {
            Expr::Array(_)
            | Expr::Map(_)
            | Expr::StructInit { .. }
            | Expr::Number(_)
            | Expr::Float(_)
            | Expr::String(_)
            | Expr::Null => true,
            Expr::Call { name, .. } => {
                let bare = name.rsplit("::").next().unwrap_or(name.as_str());
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                struct_names.contains(name)
                    || struct_names.contains(bare)
                    || fresh_fns.contains(name)
                    || fresh_fns.contains(bare)
                    || fresh_fns.contains(&name.replace("::", "__"))
                    || fresh_fns.contains(&name.replace("__", "::"))
            }
            Expr::Ternary {
                then_branch,
                else_branch,
                ..
            } => {
                fresh_expr(then_branch, struct_names, fresh_fns)
                    && fresh_expr(else_branch, struct_names, fresh_fns)
            }
            Expr::NullCoalesce { value, default } => {
                fresh_expr(value, struct_names, fresh_fns)
                    && fresh_expr(default, struct_names, fresh_fns)
            }
            Expr::ForceUnwrap(inner) => fresh_expr(inner, struct_names, fresh_fns),
            _ => false,
        }
    }
    fn walk(
        stmts: &[Stmt],
        struct_names: &std::collections::HashSet<String>,
        fresh_fns: &std::collections::HashSet<String>,
        found: &mut bool,
        ok: &mut bool,
    ) {
        for s in stmts {
            if !*ok {
                return;
            }
            match s.inner_stmt() {
                Stmt::Return(opt) => {
                    *found = true;
                    if let Some(e) = opt {
                        if !fresh_expr(e, struct_names, fresh_fns) {
                            *ok = false;
                            return;
                        }
                    }
                }
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    walk(then_block, struct_names, fresh_fns, found, ok);
                    if let Some(els) = else_block {
                        walk(els, struct_names, fresh_fns, found, ok);
                    }
                }
                Stmt::While { body, .. }
                | Stmt::Repeat { body }
                | Stmt::For { body, .. }
                | Stmt::ForEach { body, .. } => walk(body, struct_names, fresh_fns, found, ok),
                Stmt::TryCatch {
                    try_block,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    walk(try_block, struct_names, fresh_fns, found, ok);
                    walk(catch_block, struct_names, fresh_fns, found, ok);
                    if let Some(fin) = finally_block {
                        walk(fin, struct_names, fresh_fns, found, ok);
                    }
                }
                // Nested functions are separate scopes; their returns do
                // not affect the outer function.
                Stmt::Function { .. } => {}
                _ => {}
            }
        }
    }
    let mut found = false;
    let mut ok = true;
    walk(body, struct_names, fresh_fns, &mut found, &mut ok);
    found && ok
}

pub fn fn_returns_fresh_value(
    body: &[Stmt],
    struct_names: &std::collections::HashSet<String>,
) -> bool {
    let empty = std::collections::HashSet::new();
    fn_returns_fresh_value_ext(body, struct_names, &empty)
}

/// Fixed-point freshness inference over all functions in the program (alya-lang/alya#81).
/// Propagates freshness across call chains (e.g. `g()` returns `f()`, where `f()` is fresh).
/// Bounded to 32 passes to guarantee termination against any theoretical cycle.
pub fn infer_program_fresh_functions(
    statements: &[Stmt],
    struct_names: &std::collections::HashSet<String>,
) -> std::collections::HashSet<String> {
    let mut fresh_set = std::collections::HashSet::new();

    let builtin_fresh = [
        "split",
        "lines",
        "read_lines",
        "array_clone",
        "array_slice",
        "array_concat",
        "array_reverse",
        "array_unique",
        "array_sort",
        "array_chunk",
        "array_fill",
        "keys",
        "values",
        "map_entries",
        "set_to_array",
        "queue_to_array",
        "csv_parse",
        "json_parse_array",
        "parse_array",
        "map",
        "map_clone",
        "map_merge",
        "map_from_entries",
        "set_new",
        "set_from_array",
        "set_union",
        "set_intersection",
        "set_difference",
        "json_parse_object",
        "parse_object",
        "url_parse_query",
    ];
    for b in builtin_fresh {
        fresh_set.insert(b.to_string());
    }

    let mut fns: Vec<(String, &[Stmt])> = Vec::new();
    for s in statements {
        if let Stmt::Function { name, body, .. } = s.inner_stmt() {
            fns.push((name.clone(), body));
        }
    }

    let mut changed = true;
    let mut passes = 0;
    // #101: bare freshness markers need a single owner (a stale
    // freshness claim is use-after-free, alya-lang/alya#79).
    let ambiguous = ambiguous_bares_of_stmts(statements);
    while changed && passes < 32 {
        changed = false;
        passes += 1;
        for (name, body) in &fns {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if fresh_set.contains(name) && fresh_set.contains(bare) {
                continue;
            }
            if fn_returns_fresh_value_ext(body, struct_names, &fresh_set) {
                fresh_set.insert(name.clone());
                if is_simple_name(name) || !ambiguous.contains(bare) {
                    fresh_set.insert(bare.to_string());
                }
                let colon = name.replace("__", "::");
                let mangled = name.replace("::", "__");
                fresh_set.insert(colon);
                fresh_set.insert(mangled);
                changed = true;
            }
        }
    }

    fresh_set
}

/// True when a call is proven to return an integer: an explicit `-> int`
/// annotation marker, or a builtin known to produce integers.
///
/// Used to keep equality/say fast paths for known integers. The error
/// direction is safe: a false positive only preserves today's behavior.
pub fn call_returns_known_int(name: &str, vars: &HashMap<String, VarType>) -> bool {
    let bare = name.rsplit("::").next().unwrap_or(name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    if matches!(
        bare,
        "int"
            | "len"
            | "arr_len"
            | "ord"
            | "time"
            | "clock_ms"
            | "rand"
            | "rand_int"
            | "abs"
            | "abs_val"
            // Bitwise builtins are int->int word ops (logical shifts):
            // results are machine words, never heap pointers.
            | "bit_and"
            | "bit_or"
            | "bit_xor"
            | "bit_shl"
            | "bit_shr"
            | "bit_not"
    ) {
        return true;
    }
    vars.contains_key(&format!("fn_ret_int:{}", name))
        // #101: qualified-spelled calls consult exact markers only.
        || (is_simple_name(name)
            && (vars.contains_key(&format!("fn_ret_int:{}", bare))
                || vars.keys().any(|k| {
                    k.starts_with("fn_ret_int:")
                        && (k.ends_with(&format!("__{}", bare))
                            || k.ends_with(&format!("::{}", bare)))
                })))
}

/// True when an `==`/`!=` operand has no proven static type for equality:
/// not a literal, not proven string/float/map/array, and not a proven
/// integer (annotated variable or known int-returning call).
///
/// Callers exclude literals and typed expressions first; what remains is
/// array indexing into untyped collections, untyped locals/params, struct
/// fields, and calls with unknown returns. Such operands are classified
/// at runtime (string -> content compare, otherwise -> word compare).
pub fn eq_operand_is_dynamic(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Identifier(name) => !vars.contains_key(&format!("var_is_int:{}", name)),
        Expr::Call { name, .. } => !call_returns_known_int(name, vars),
        _ => true,
    }
}

/// Non-escaping heap params for the probe-free fast path.
///
/// Returns the params that are never written (no `=`-assign or
/// `let`-rebind to the name) and never placed where the callee frame's
/// ownership share would be needed: returned, thrown, stored into a
/// container slot or a global, or mentioned inside a nested function
/// (closures may outlive the frame; mentions are textual, so shadowing
/// only over-approximates). Reads — arithmetic, comparisons, index and
/// field loads, call arguments (callees retain their own share),
/// loop iterables — are all borrows: the caller's slot outlives the
/// call, so no share is needed and both the entry retain and the scope
/// release can be skipped as a balanced pair.
///
/// Soundness notes: interior aliases (`local = param`) retain their own
/// share through the alias machinery, so they are reads here. Loops
/// and conditionals only matter through the statements they contain.
/// There are no references, address-of, or eval, so the statement forms
/// below are the complete set of writers/escapers.
pub fn nonescaping_params(
    body: &[&Stmt],
    params: &[String],
    globals: &std::collections::HashSet<String>,
) -> HashSet<String> {
    use std::collections::HashSet as Set;
    let mut escaping: Set<String> = Set::new();

    fn mentions_any(expr: &Expr, params: &[String]) -> Vec<String> {
        params
            .iter()
            .filter(|p| expr_mentions_var(expr, p))
            .cloned()
            .collect()
    }

    fn walk_block(
        stmts: &[Stmt],
        params: &[String],
        globals: &Set<String>,
        escaping: &mut Set<String>,
    ) {
        let refs: Vec<&Stmt> = stmts.iter().collect();
        walk(&refs, params, globals, escaping);
    }

    fn walk(stmts: &[&Stmt], params: &[String], globals: &Set<String>, escaping: &mut Set<String>) {
        for stmt in stmts {
            match *stmt {
                Stmt::Let { name, .. } | Stmt::Const { name, .. } => {
                    // A `let` with a param's name rebinds (releasing) it.
                    if params.contains(name) {
                        escaping.insert(name.clone());
                    }
                }
                Stmt::Assign { name, value } => {
                    if params.contains(name) {
                        escaping.insert(name.clone());
                    }
                    // Storing a param into a global outlives the frame
                    // (globals have no retaining store path); storing
                    // into a local aliases through the retaining alias
                    // machinery and stays a borrow.
                    if globals.contains(name) {
                        for p in mentions_any(value, params) {
                            escaping.insert(p);
                        }
                    }
                }
                Stmt::IndexAssign { value, .. } | Stmt::FieldAssign { value, .. } => {
                    // Container-slot stores outlive the frame; the
                    // set-path retain covers locals, but treat any
                    // param store conservatively as escaping.
                    for p in mentions_any(value, params) {
                        escaping.insert(p);
                    }
                }
                Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) => {
                    for p in mentions_any(expr, params) {
                        escaping.insert(p);
                    }
                }
                Stmt::Function { body, .. } => {
                    // Nested closures may outlive the frame: any textual
                    // mention escapes (shadowing only over-approximates).
                    for s in body.iter() {
                        for p in params {
                            if stmt_mentions(s, p) {
                                escaping.insert(p.clone());
                            }
                        }
                    }
                }
                Stmt::For { body, .. } => walk_block(body, params, globals, escaping),
                Stmt::ForEach { body, .. } => walk_block(body, params, globals, escaping),
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    walk_block(then_block, params, globals, escaping);
                    if let Some(eb) = else_block {
                        walk_block(eb, params, globals, escaping);
                    }
                }
                Stmt::While { body, .. } | Stmt::Repeat { body } => {
                    walk_block(body, params, globals, escaping)
                }
                Stmt::TryCatch {
                    try_block,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    walk_block(try_block, params, globals, escaping);
                    walk_block(catch_block, params, globals, escaping);
                    if let Some(fb) = finally_block {
                        walk_block(fb, params, globals, escaping);
                    }
                }
                Stmt::Defer(inner) | Stmt::Pub(inner) => walk(
                    std::slice::from_ref(&inner.as_ref()),
                    params,
                    globals,
                    escaping,
                ),
                // Pure reads and control transfers: Say, Expr (call args
                // are retained by callees; in-place mutation needs no
                // share), Break/Continue, definitions, imports/externs.
                Stmt::Import { .. }
                | Stmt::ExternBlock { .. }
                | Stmt::Say(_)
                | Stmt::StructDef { .. }
                | Stmt::EnumDef { .. }
                | Stmt::Return(None)
                | Stmt::Break
                | Stmt::Continue
                | Stmt::Expr(_)
                | Stmt::Throw(None)
                | Stmt::InterfaceDef { .. } => {}
            }
        }
    }

    fn stmt_mentions(stmt: &Stmt, target: &str) -> bool {
        match stmt {
            Stmt::Say(expr) | Stmt::Expr(expr) => expr_mentions_var(expr, target),
            Stmt::Let { value, .. } | Stmt::Const { name: _, value } => {
                expr_mentions_var(value, target)
            }
            Stmt::Assign { value, .. } => expr_mentions_var(value, target),
            Stmt::IndexAssign {
                array,
                index,
                value,
            } => {
                expr_mentions_var(array, target)
                    || expr_mentions_var(index, target)
                    || expr_mentions_var(value, target)
            }
            Stmt::FieldAssign { object, value, .. } => {
                expr_mentions_var(object, target) || expr_mentions_var(value, target)
            }
            Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) => expr_mentions_var(expr, target),
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                expr_mentions_var(condition, target)
                    || then_block.iter().any(|s| stmt_mentions(s, target))
                    || else_block
                        .as_ref()
                        .is_some_and(|eb| eb.iter().any(|s| stmt_mentions(s, target)))
            }
            Stmt::While { condition, body } => {
                expr_mentions_var(condition, target)
                    || body.iter().any(|s| stmt_mentions(s, target))
            }
            Stmt::Repeat { body } => body.iter().any(|s| stmt_mentions(s, target)),
            Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
                body.iter().any(|s| stmt_mentions(s, target))
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                try_block.iter().any(|s| stmt_mentions(s, target))
                    || catch_block.iter().any(|s| stmt_mentions(s, target))
                    || finally_block
                        .as_ref()
                        .is_some_and(|fb| fb.iter().any(|s| stmt_mentions(s, target)))
            }
            Stmt::Defer(inner) | Stmt::Pub(inner) => stmt_mentions(inner, target),
            Stmt::Function { body, .. } => body.iter().any(|s| stmt_mentions(s, target)),
            _ => false,
        }
    }

    walk(body, params, globals, &mut escaping);
    params
        .iter()
        .filter(|p| !escaping.contains(*p))
        .cloned()
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;

    fn test_vars() -> HashMap<String, VarType> {
        let mut vars = HashMap::new();
        vars.insert("s".to_string(), VarType::StringOffset(8));
        vars.insert("n".to_string(), VarType::Number(16));
        vars
    }

    #[test]
    fn string_store_needs_dup_only_for_nonliteral_strings() {
        let vars = test_vars();
        // Literals live in rodata: never duplicated.
        assert!(!string_store_needs_dup(&Expr::String("lit".into()), &vars));
        // Concat results and identifiers live in the ring: duplicate.
        assert!(string_store_needs_dup(
            &Expr::Binary {
                left: Box::new(Expr::Identifier("s".into())),
                op: crate::ast::BinaryOp::Add,
                right: Box::new(Expr::String("x".into())),
            },
            &vars
        ));
        assert!(string_store_needs_dup(&Expr::Identifier("s".into()), &vars));
        // Non-strings never duplicate.
        assert!(!string_store_needs_dup(
            &Expr::Identifier("n".into()),
            &vars
        ));
        assert!(!string_store_needs_dup(&Expr::Number(1), &vars));
        // Explicitly heap-owned clones keep their free pairing: never duplicate.
        for name in ["str_clone", "string_clone"] {
            assert!(
                !string_store_needs_dup(
                    &Expr::Call {
                        name: name.to_string(),
                        args: vec![Expr::Identifier("s".into())],
                    },
                    &vars
                ),
                "{}",
                name
            );
        }
    }
}
