use super::common::collect_function_defs;
use crate::ast::*;
use crate::codegen::analysis::predicates::{is_simple_name, seed_ambiguity_markers};
use crate::codegen::analysis::traversal::CallIndex;
use crate::may_record_bare;
use std::collections::{HashMap, HashSet};

fn expr_is_definitely_string(expr: &Expr, known_strings: &HashSet<String>) -> bool {
    match expr {
        Expr::String(_) | Expr::InterpolatedString(_) => true,
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
            if bare == "slice" {
                return !args.is_empty() && expr_is_definitely_string(&args[0], known_strings);
            }
            if bare == "get"
                && args.len() >= 3
                && expr_is_definitely_string(&args[2], known_strings)
            {
                return true;
            }
            known_strings.contains(&format!("fn_ret_str:{}", name))
                // #101: qualified-spelled calls consult exact markers
                // only; bare/suffix fallbacks would inherit markers set
                // by an unrelated same-bare function.
                || (is_simple_name(name)
                    && (known_strings.contains(&format!("fn_ret_str:{}", bare))
                        || known_strings.iter().any(|k| {
                            k.starts_with("fn_ret_str:")
                                && (k.ends_with(&format!("__{}", bare))
                                    || k.ends_with(&format!("::{}", bare)))
                        })))
        }
        Expr::Identifier(name) => known_strings.contains(name),
        Expr::Binary {
            left,
            op: BinaryOp::Add,
            right,
        } => {
            expr_is_definitely_string(left, known_strings)
                || expr_is_definitely_string(right, known_strings)
        }
        Expr::FieldAccess { object, field } => {
            if let Expr::Identifier(obj_name) = &**object {
                if known_strings.contains(&format!("{}.{}", obj_name, field)) {
                    return true;
                }
            }
            known_strings.contains(&format!("struct_field_str:{}", field))
        }
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => {
            // Both arms must be strings (mirror the codegen predicate):
            // with mixed arms the result is dynamic. Null arms are
            // transparent (a `when` without `else` desugars to Null).
            let tb = expr_is_definitely_string(then_branch, known_strings);
            let eb = expr_is_definitely_string(else_branch, known_strings);
            let tn = matches!(**then_branch, Expr::Null);
            let en = matches!(**else_branch, Expr::Null);
            (tb || tn) && (eb || en) && (tb || eb)
        }
        Expr::NullCoalesce { value, default } => {
            let vb = expr_is_definitely_string(value, known_strings);
            let db = expr_is_definitely_string(default, known_strings);
            let vn = matches!(**value, Expr::Null);
            let dn = matches!(**default, Expr::Null);
            (vb || vn) && (db || dn) && (vb || db)
        }
        Expr::Index { array, index } => {
            if let Expr::String(field) = &**index {
                if known_strings.contains(&format!("map_field_str:{}", field)) {
                    return true;
                }
            }
            if let (Expr::Identifier(map_name), Expr::String(field)) = (&**array, &**index) {
                if known_strings.contains(&format!("map_str:{}.{}", map_name, field)) {
                    return true;
                }
            }
            if matches!(**index, Expr::String(_)) || expr_is_definitely_string(index, known_strings)
            {
                return false;
            }
            if let (Expr::Identifier(arr_name), Expr::Number(idx)) = (&**array, &**index) {
                let idx_usize = *idx as usize;
                if known_strings.contains(&format!("tuple_elem_str:{}:{}", arr_name, idx_usize)) {
                    return true;
                }
            }
            if expr_is_string_array(array, known_strings) {
                return true;
            }
            match &**array {
                Expr::Identifier(arr_name) => {
                    (known_strings.contains(&format!("arr_is_str:{}", arr_name))
                        && !known_strings.contains(&format!("arr_nonstr:{}", arr_name)))
                        || known_strings.contains(arr_name)
                }
                Expr::Call { name, args } => {
                    let bare = name.rsplit("::").next().unwrap_or(name.as_str());
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    // `split` is absent from the static list on purpose
                    // (`Tensor.split` and any user `split` return
                    // non-string arrays), but a `split` call on a proven
                    // string is the string builtin: its pieces are
                    // strings (alya-lang/alya#102). Without this,
                    // dynamic-container reads keyed by split pieces
                    // misroute to array indexing and fault.
                    (bare == "split"
                        && !args.is_empty()
                        && expr_is_definitely_string(&args[0], known_strings))
                        || matches!(bare, "args" | "cli_args" | "lines" | "read_lines" | "keys")
                        || known_strings.contains(&format!("fn_ret_str_arr:{}", name))
                        // #101: qualified-spelled calls consult exact only.
                        || (is_simple_name(name)
                            && known_strings.contains(&format!("fn_ret_str_arr:{}", bare)))
                }
                _ => expr_is_definitely_string(array, known_strings),
            }
        }
        Expr::ForceUnwrap(inner) => expr_is_definitely_string(inner, known_strings),
        _ => false,
    }
}

fn expr_is_string_array(expr: &Expr, known_strings: &HashSet<String>) -> bool {
    match expr {
        Expr::Array(elems) => {
            !elems.is_empty()
                && elems
                    .iter()
                    .all(|e| expr_is_definitely_string(e, known_strings))
        }
        Expr::Identifier(name) => {
            known_strings.contains(&format!("arr_is_str:{}", name))
                && !known_strings.contains(&format!("arr_nonstr:{}", name))
        }
        Expr::Call { name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            // NOTE: `split` is deliberately absent from the static list
            // (see above); the string-receiver carve-out applies here too.
            (bare == "split"
                && !args.is_empty()
                && expr_is_definitely_string(&args[0], known_strings))
                || matches!(bare, "args" | "cli_args" | "lines" | "read_lines" | "keys")
                || known_strings.contains(&format!("fn_ret_str_arr:{}", name))
                // #101: qualified-spelled calls consult exact only.
                || (is_simple_name(name)
                    && known_strings.contains(&format!("fn_ret_str_arr:{}", bare)))
        }
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => {
            // Both arms must be string arrays; otherwise the outcome is
            // dynamic and callers must not assume string elements.
            expr_is_string_array(then_branch, known_strings)
                && expr_is_string_array(else_branch, known_strings)
        }
        Expr::NullCoalesce { value, default } => {
            expr_is_string_array(value, known_strings)
                && expr_is_string_array(default, known_strings)
        }
        Expr::ForceUnwrap(inner) => expr_is_string_array(inner, known_strings),
        _ => false,
    }
}

fn stmts_return_string(stmts: &[Stmt], known_strings: &HashSet<String>) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::Return(Some(expr)) => expr_is_definitely_string(expr, known_strings),
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            stmts_return_string(then_block, known_strings)
                || else_block
                    .as_ref()
                    .is_some_and(|eb| stmts_return_string(eb, known_strings))
        }
        Stmt::While { body, .. }
        | Stmt::Repeat { body }
        | Stmt::For { body, .. }
        | Stmt::ForEach { body, .. } => stmts_return_string(body, known_strings),
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            stmts_return_string(try_block, known_strings)
                || stmts_return_string(catch_block, known_strings)
                || finally_block
                    .as_ref()
                    .is_some_and(|fb| stmts_return_string(fb, known_strings))
        }
        _ => false,
    })
}

fn collect_tuple_returns_string(
    stmts: &[Stmt],
    known_strings: &HashSet<String>,
    fn_name: &str,
    target_strings: &mut HashSet<String>,
) {
    for s in stmts {
        match s {
            Stmt::Return(Some(Expr::Array(elements))) => {
                for (i, elem) in elements.iter().enumerate() {
                    if expr_is_definitely_string(elem, known_strings) {
                        target_strings.insert(format!("fn_ret_tuple_str:{}:{}", fn_name, i));
                        // Presence summary for the scan gate above.
                        target_strings.insert(format!("fn_ret_tuple:{}", fn_name));
                    }
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_tuple_returns_string(then_block, known_strings, fn_name, target_strings);
                if let Some(eb) = else_block {
                    collect_tuple_returns_string(eb, known_strings, fn_name, target_strings);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                collect_tuple_returns_string(body, known_strings, fn_name, target_strings);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_tuple_returns_string(try_block, known_strings, fn_name, target_strings);
                collect_tuple_returns_string(catch_block, known_strings, fn_name, target_strings);
                if let Some(fb) = finally_block {
                    collect_tuple_returns_string(fb, known_strings, fn_name, target_strings);
                }
            }
            _ => {}
        }
    }
}

/// True when a body returns a plain (non-array) string literal on some
/// path (alya-lang/alya#107): such a function doesn't exclusively
/// return string arrays, so string-array positives must not fire for
/// it. Mirrors the `returns_non_string_lit` walker shape (literals
/// only; identifiers/calls can't prove a plain string here — and the
/// scalar veto already covers proven non-strings).
fn body_returns_plain_string(stmts: &[Stmt]) -> bool {
    for s in stmts {
        match s {
            Stmt::Return(Some(Expr::String(_) | Expr::InterpolatedString(_))) => {
                return true;
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                if body_returns_plain_string(then_block) {
                    return true;
                }
                if let Some(eb) = else_block {
                    if body_returns_plain_string(eb) {
                        return true;
                    }
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                if body_returns_plain_string(body) {
                    return true;
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                if body_returns_plain_string(try_block)
                    || body_returns_plain_string(catch_block)
                    || finally_block
                        .as_ref()
                        .is_some_and(|fb| body_returns_plain_string(fb))
                {
                    return true;
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                if body_returns_plain_string(std::slice::from_ref(inner)) {
                    return true;
                }
            }
            // Nested functions own their returns; do not descend.
            Stmt::Function { .. } => {}
            _ => {}
        }
    }
    false
}

fn collect_function_returns_string_array(
    stmts: &[Stmt],
    known_strings: &HashSet<String>,
    fn_name: &str,
    target_strings: &mut HashSet<String>,
) {
    // #107: string-array positives need veto gating like scalar
    // positives (`ret_vetoed`): a non-string path — or a plain-string
    // (non-array) path — means the function doesn't exclusively
    // return string arrays, so no marker. (Vetoes precede positives
    // via the closure pre-seed, so no stale markers.)
    if known_strings.contains(&format!("fn_ret_nonstr:{}", fn_name))
        || body_returns_plain_string(stmts)
    {
        return;
    }
    for s in stmts {
        match s {
            Stmt::Return(Some(expr)) if expr_is_string_array(expr, known_strings) => {
                target_strings.insert(format!("fn_ret_str_arr:{}", fn_name));
                let bare = fn_name.rsplit("::").next().unwrap_or(fn_name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                // #101: bare markers need a single owner.
                if may_record_bare!(known_strings, fn_name, bare) {
                    target_strings.insert(format!("fn_ret_str_arr:{}", bare));
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_function_returns_string_array(
                    then_block,
                    known_strings,
                    fn_name,
                    target_strings,
                );
                if let Some(eb) = else_block {
                    collect_function_returns_string_array(
                        eb,
                        known_strings,
                        fn_name,
                        target_strings,
                    );
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                collect_function_returns_string_array(body, known_strings, fn_name, target_strings);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_function_returns_string_array(
                    try_block,
                    known_strings,
                    fn_name,
                    target_strings,
                );
                collect_function_returns_string_array(
                    catch_block,
                    known_strings,
                    fn_name,
                    target_strings,
                );
                if let Some(fb) = finally_block {
                    collect_function_returns_string_array(
                        fb,
                        known_strings,
                        fn_name,
                        target_strings,
                    );
                }
            }
            _ => {}
        }
    }
}

fn collect_struct_defs(stmts: &[Stmt], map: &mut HashMap<String, Vec<String>>) {
    for s in stmts {
        match s {
            Stmt::StructDef { name, fields, .. } => {
                map.insert(name.clone(), fields.clone());
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_struct_defs(then_block, map);
                if let Some(eb) = else_block {
                    collect_struct_defs(eb, map);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. }
            | Stmt::Function { body, .. } => {
                collect_struct_defs(body, map);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_struct_defs(try_block, map);
                collect_struct_defs(catch_block, map);
                if let Some(fb) = finally_block {
                    collect_struct_defs(fb, map);
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_struct_defs(std::slice::from_ref(inner), map);
            }
            _ => {}
        }
    }
}

/// Literals that are provably not strings, independent of scope. Used
/// as negative evidence against existential `fn_param_str` positives:
/// one such call arg proves the parameter is dynamic. Dynamic-typed
/// args (identifiers, calls) prove nothing either way and are ignored.
fn expr_is_definitely_non_string_lit(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Number(_)
            | Expr::Float(_)
            | Expr::Array(_)
            | Expr::Map(_)
            | Expr::Null
            | Expr::StructInit { .. }
    )
}

fn scan_expr_for_strings(
    expr: &Expr,
    struct_defs: &HashMap<String, Vec<String>>,
    known_strings: &mut HashSet<String>,
    conflicts: &HashSet<String>,
) {
    match expr {
        Expr::Call { name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if (bare == "push" || bare == "array_push" || bare == "append") && args.len() >= 2 {
                if let Expr::Identifier(arr_name) = &args[0] {
                    if expr_is_definitely_string(&args[1], known_strings) {
                        known_strings.insert(format!("arr_is_str:{}", arr_name));
                    } else {
                        // Mixed content voids whole-array string claims
                        // (alya-lang/alya#39): readers fall back to
                        // slot-kind dispatch instead of `%s` on non-strings.
                        known_strings.insert(format!("arr_nonstr:{}", arr_name));
                    }
                }
            }
            // Per-call param evidence (scope-aware: caller locals are
            // visible here). Positives are existential and MUST be gated
            // by negatives at consumers: one literal non-string call
            // proves the parameter is dynamic, so trusting a positive
            // from another call would fold `is string` to constant-true
            // and miscompile every other call (`%s` on an int segfaults).
            for (idx, arg) in args.iter().enumerate() {
                if expr_is_definitely_string(arg, known_strings) {
                    known_strings.insert(format!("fn_param_str:{}:{}", name, idx));
                    // #101: bare markers need a single owner.
                    if may_record_bare!(known_strings, name, bare) {
                        known_strings.insert(format!("fn_param_str:{}:{}", bare, idx));
                    }
                } else if expr_is_definitely_non_string_lit(arg) {
                    known_strings.insert(format!("fn_param_nonstr:{}:{}", name, idx));
                    known_strings.insert(format!("fn_param_nonstr:{}:{}", bare, idx));
                }
            }
            if let Some(fields) = struct_defs.get(name) {
                for (i, arg) in args.iter().enumerate() {
                    if expr_is_definitely_string(arg, known_strings) {
                        if let Some(fname) = fields.get(i) {
                            if !struct_field_markers_mixed(known_strings, name, fname) {
                                known_strings
                                    .insert(format!("struct_field_str:{}.{}", name, fname));
                                if !conflicts.contains(fname) {
                                    known_strings.insert(format!("struct_field_str:{}", fname));
                                }
                            }
                        }
                    }
                }
            }
            for arg in args {
                scan_expr_for_strings(arg, struct_defs, known_strings, conflicts);
            }
        }
        Expr::StructInit { name, fields } => {
            for (fname, fval) in fields {
                if expr_is_definitely_string(fval, known_strings)
                    && !struct_field_markers_mixed(known_strings, name, fname)
                {
                    known_strings.insert(format!("struct_field_str:{}.{}", name, fname));
                    if !conflicts.contains(fname) {
                        known_strings.insert(format!("struct_field_str:{}", fname));
                    }
                }
                scan_expr_for_strings(fval, struct_defs, known_strings, conflicts);
            }
        }
        Expr::Binary { left, right, .. } => {
            scan_expr_for_strings(left, struct_defs, known_strings, conflicts);
            scan_expr_for_strings(right, struct_defs, known_strings, conflicts);
        }
        Expr::Unary { expr, .. } => {
            scan_expr_for_strings(expr, struct_defs, known_strings, conflicts);
        }
        Expr::ForceUnwrap(inner) => {
            scan_expr_for_strings(inner, struct_defs, known_strings, conflicts);
        }
        Expr::Array(elems) => {
            for elem in elems {
                scan_expr_for_strings(elem, struct_defs, known_strings, conflicts);
            }
        }
        Expr::Index { array, index } => {
            scan_expr_for_strings(array, struct_defs, known_strings, conflicts);
            scan_expr_for_strings(index, struct_defs, known_strings, conflicts);
        }
        Expr::FieldAccess { object, .. } => {
            scan_expr_for_strings(object, struct_defs, known_strings, conflicts);
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                scan_expr_for_strings(k, struct_defs, known_strings, conflicts);
                scan_expr_for_strings(v, struct_defs, known_strings, conflicts);
            }
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                scan_expr_for_strings(part, struct_defs, known_strings, conflicts);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            scan_expr_for_strings(condition, struct_defs, known_strings, conflicts);
            scan_expr_for_strings(then_branch, struct_defs, known_strings, conflicts);
            scan_expr_for_strings(else_branch, struct_defs, known_strings, conflicts);
        }
        Expr::NullCoalesce { value, default } => {
            scan_expr_for_strings(value, struct_defs, known_strings, conflicts);
            scan_expr_for_strings(default, struct_defs, known_strings, conflicts);
        }
        _ => {}
    }
}

/// Return-position veto: a literal non-string, non-null `return`
/// (number, float, array, map, struct) proves the function is dynamic
/// no matter what string paths it also has. Null returns are exempt:
/// null (0) passes `str_store` through unharmed, so string-or-null
/// functions keep their marker (stable dups for strings) while mixed
/// functions (e.g. json `_parser_parse_val`, which also returns ints)
/// stay dynamic. Without this, an existential `fn_ret_str` positive
/// folds mixed functions to constant-string and
/// `string_store_needs_dup` emits `str_store` for heap results, copying
/// map memory as string bytes (silent corruption). Mirrors the
/// `fn_param_nonstr` argument vetoes; only adds vetoes.
fn record_return_nonstr_vetoes(
    func_name: &str,
    body: &[Stmt],
    known_strings: &mut HashSet<String>,
) {
    // Builtins provably returning non-strings (mirror the codebase's
    // own classifications: `call_returns_known_int` plus the float
    // builtins). A `return int(...)` proves the function dynamic just
    // like a literal does. Simple spellings only: qualified calls
    // resolve through their own markers. (User shadowing over-vetoes
    // soundly — vetoes only ever degrade to dynamic dispatch.)
    fn is_non_string_builtin(name: &str) -> bool {
        matches!(
            name,
            "int"
                | "to_int"
                | "parse_int"
                | "len"
                | "arr_len"
                | "ord"
                | "time"
                | "clock_ms"
                | "rand"
                | "rand_int"
                | "abs"
                | "abs_val"
                | "bit_and"
                | "bit_or"
                | "bit_xor"
                | "bit_shl"
                | "bit_shr"
                | "bit_not"
                | "float"
                | "to_float"
                | "parse_float"
        )
    }
    // Transitive veto through already-vetoed callees: `return F(...)`
    // where F is proven dynamic (any spelling; over-matching is sound
    // — vetoes only degrade). Consults the accumulating set, so this
    // propagates one call level per fixpoint round (see the closure
    // pre-seed below).
    fn returns_vetoed_call(stmts: &[Stmt], known_strings: &HashSet<String>) -> bool {
        for s in stmts {
            match s {
                Stmt::Return(Some(Expr::Call { name, .. })) => {
                    let bare = name.rsplit("::").next().unwrap_or(name.as_str());
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    let swapped_us = name.replace("::", "__");
                    let swapped_col = name.replace("__", "::");
                    if known_strings.contains(&format!("fn_ret_nonstr:{}", name))
                        || known_strings.contains(&format!("fn_ret_nonstr:{}", bare))
                        || known_strings.contains(&format!("fn_ret_nonstr:{}", swapped_us))
                        || known_strings.contains(&format!("fn_ret_nonstr:{}", swapped_col))
                    {
                        return true;
                    }
                    if !name.contains("::") && !name.contains("__") && is_non_string_builtin(name) {
                        return true;
                    }
                }
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if returns_vetoed_call(then_block, known_strings) {
                        return true;
                    }
                    if let Some(eb) = else_block {
                        if returns_vetoed_call(eb, known_strings) {
                            return true;
                        }
                    }
                }
                Stmt::While { body, .. }
                | Stmt::Repeat { body }
                | Stmt::For { body, .. }
                | Stmt::ForEach { body, .. } => {
                    if returns_vetoed_call(body, known_strings) {
                        return true;
                    }
                }
                Stmt::TryCatch {
                    try_block,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    if returns_vetoed_call(try_block, known_strings)
                        || returns_vetoed_call(catch_block, known_strings)
                        || finally_block
                            .as_ref()
                            .is_some_and(|fb| returns_vetoed_call(fb, known_strings))
                    {
                        return true;
                    }
                }
                Stmt::Pub(inner) | Stmt::Defer(inner) => {
                    if returns_vetoed_call(std::slice::from_ref(inner), known_strings) {
                        return true;
                    }
                }
                // Nested functions own their returns; do not descend.
                Stmt::Function { .. } => {}
                _ => {}
            }
        }
        false
    }
    fn returns_non_string_lit(stmts: &[Stmt]) -> bool {
        for s in stmts {
            match s {
                Stmt::Return(Some(e)) => {
                    if matches!(
                        e,
                        Expr::Number(_)
                            | Expr::Float(_)
                            | Expr::Array(_)
                            | Expr::Map(_)
                            | Expr::Null
                            | Expr::StructInit { .. }
                    ) {
                        return true;
                    }
                }
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if returns_non_string_lit(then_block) {
                        return true;
                    }
                    if let Some(eb) = else_block {
                        if returns_non_string_lit(eb) {
                            return true;
                        }
                    }
                }
                Stmt::While { body, .. }
                | Stmt::Repeat { body }
                | Stmt::For { body, .. }
                | Stmt::ForEach { body, .. } => {
                    if returns_non_string_lit(body) {
                        return true;
                    }
                }
                Stmt::TryCatch {
                    try_block,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    if returns_non_string_lit(try_block)
                        || returns_non_string_lit(catch_block)
                        || finally_block
                            .as_ref()
                            .is_some_and(|fb| returns_non_string_lit(fb))
                    {
                        return true;
                    }
                }
                Stmt::Pub(inner) | Stmt::Defer(inner) => {
                    if returns_non_string_lit(std::slice::from_ref(inner)) {
                        return true;
                    }
                }
                // Nested functions own their returns; do not descend.
                Stmt::Function { .. } => {}
                _ => {}
            }
        }
        false
    }
    if returns_non_string_lit(body) || returns_vetoed_call(body, known_strings) {
        let bare = func_name.rsplit("::").next().unwrap_or(func_name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        known_strings.insert(format!("fn_ret_nonstr:{}", func_name));
        // Methods (`Type__method`) must not veto the bare free-function
        // name: e.g. `Box.touch` returning ints would otherwise kill the
        // unrelated bare `touch` string marker (e2e arity collision).
        if !func_name.contains("__") {
            known_strings.insert(format!("fn_ret_nonstr:{}", bare));
        }
    }
}

/// Transitive veto through same-arg forwarding: when F forwards its own
/// proven-dynamic parameter (one with a direct literal non-string call,
/// i.e. an `fn_param_nonstr` veto) into G, G provably receives non-strings
/// too. Without this, a typed sibling (e.g. cache `set_str` with a
/// `: string` value) plants an existential `fn_param_str` positive on the
/// shared helper (`store_set`) while the dynamic calls arrive through the
/// untyped wrapper (`set`) and never veto the helper directly — the
/// helper's parameter folds to constant-string, map stores skip the
/// retain, and heap values corrupt (use-after-free). Runs inside the
/// fixpoint so forwarding chains converge; only adds vetoes.
fn record_forwarding_param_vetoes(
    func_name: &str,
    params: &[String],
    body: &[Stmt],
    known_strings: &mut HashSet<String>,
) {
    let bare_f = func_name.rsplit("::").next().unwrap_or(func_name);
    let bare_f = bare_f.rsplit("__").next().unwrap_or(bare_f);
    let mut vetoed_idx: Vec<bool> = vec![false; params.len()];
    let mut any_vetoed = false;
    for (j, slot) in vetoed_idx.iter_mut().enumerate() {
        if known_strings.contains(&format!("fn_param_nonstr:{}:{}", func_name, j))
            || known_strings.contains(&format!("fn_param_nonstr:{}:{}", bare_f, j))
        {
            *slot = true;
            any_vetoed = true;
        }
    }
    if !any_vetoed {
        return;
    }
    let mut calls: Vec<(&str, String, usize, &Expr)> = Vec::new();
    collect_body_calls(body, &mut calls);
    for (callee, bare_callee, k, arg) in calls {
        if let Expr::Identifier(pname) = arg {
            if let Some(j) = params.iter().position(|p| p == pname) {
                if vetoed_idx[j] {
                    known_strings.insert(format!("fn_param_nonstr:{}:{}", callee, k));
                    known_strings.insert(format!("fn_param_nonstr:{}:{}", bare_callee, k));
                }
            }
        }
    }
}

/// Collects every (callee, callee-bare, arg-position, arg) call in a
/// function body for forwarding analysis. Does not descend into nested
/// function definitions (different parameter scope).
fn collect_body_calls<'a>(stmts: &'a [Stmt], out: &mut Vec<(&'a str, String, usize, &'a Expr)>) {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(e) | Stmt::Say(e) => collect_expr_calls(e, out),
            Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Const { value, .. } => {
                collect_expr_calls(value, out)
            }
            Stmt::Return(opt) | Stmt::Throw(opt) => {
                if let Some(e) = opt {
                    collect_expr_calls(e, out);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_expr_calls(condition, out);
                collect_body_calls(then_block, out);
                if let Some(eb) = else_block {
                    collect_body_calls(eb, out);
                }
            }
            Stmt::While { condition, body } => {
                collect_expr_calls(condition, out);
                collect_body_calls(body, out);
            }
            Stmt::Repeat { body } | Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
                collect_body_calls(body, out)
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_body_calls(try_block, out);
                collect_body_calls(catch_block, out);
                if let Some(fb) = finally_block {
                    collect_body_calls(fb, out);
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_body_calls(std::slice::from_ref(inner), out)
            }
            Stmt::Function { .. } => {}
            _ => {}
        }
    }
}

fn collect_expr_calls<'a>(expr: &'a Expr, out: &mut Vec<(&'a str, String, usize, &'a Expr)>) {
    match expr {
        Expr::Call { name, args } | Expr::OptionalCall { callee: name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            for (k, arg) in args.iter().enumerate() {
                out.push((name.as_str(), bare.to_string(), k, arg));
            }
            for arg in args {
                collect_expr_calls(arg, out);
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_calls(left, out);
            collect_expr_calls(right, out);
        }
        Expr::Unary { expr, .. } | Expr::ForceUnwrap(expr) => collect_expr_calls(expr, out),
        Expr::Array(elems) => {
            for elem in elems {
                collect_expr_calls(elem, out);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            collect_expr_calls(array, out);
            collect_expr_calls(index, out);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            collect_expr_calls(object, out)
        }
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                collect_expr_calls(val, out);
            }
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                collect_expr_calls(k, out);
                collect_expr_calls(v, out);
            }
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                collect_expr_calls(part, out);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_calls(condition, out);
            collect_expr_calls(then_branch, out);
            collect_expr_calls(else_branch, out);
        }
        Expr::NullCoalesce { value, default } => {
            collect_expr_calls(value, out);
            collect_expr_calls(default, out);
        }
        Expr::Cast { expr: inner, .. } => collect_expr_calls(inner, out),
        _ => {}
    }
}

fn collect_string_vars_from_stmts(
    stmts: &[Stmt],
    struct_defs: &HashMap<String, Vec<String>>,
    known_strings: &mut HashSet<String>,
    conflicts: &HashSet<String>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let {
                name,
                type_ann,
                value,
            } => {
                if let Some(t) = type_ann {
                    if t == "string" || t == "str" {
                        known_strings.insert(name.clone());
                    } else if t == "string[]" || t == "str[]" {
                        known_strings.insert(format!("arr_is_str:{}", name));
                    }
                }
                scan_expr_for_strings(value, struct_defs, known_strings, conflicts);
                if expr_is_definitely_string(value, known_strings) {
                    known_strings.insert(name.clone());
                }
                if expr_is_string_array(value, known_strings) {
                    known_strings.insert(format!("arr_is_str:{}", name));
                }
                if let Expr::Identifier(id) = value {
                    let bare_id = id.rsplit("::").next().unwrap_or(id.as_str());
                    let bare_id = bare_id.rsplit("__").next().unwrap_or(bare_id);
                    if known_strings.contains(&format!("fn_ret_str:{}", id))
                        // #101: qualified-spelled references consult exact only.
                        || (is_simple_name(id)
                            && known_strings.contains(&format!("fn_ret_str:{}", bare_id)))
                    {
                        known_strings.insert(format!("fn_ret_str:{}", name));
                    }
                }
                if let Expr::Call { name: cname, .. } = value {
                    let bare = cname.rsplit("::").next().unwrap_or(cname.as_str());
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    let prefix1 = format!("fn_ret_tuple_str:{}:", cname);
                    let prefix2 = format!("fn_ret_tuple_str:{}:", bare);
                    // Presence summaries (exact: set alongside the marker
                    // inserts below): skip the full-set scan unless a
                    // matching marker exists.
                    if known_strings.contains(&format!("fn_ret_tuple:{}", cname))
                        || known_strings.contains(&format!("fn_ret_tuple:{}", bare))
                    {
                        let mut pending: Vec<String> = Vec::new();
                        for item in known_strings.iter() {
                            if item.starts_with(&prefix1) {
                                if let Some(idx_str) = item.strip_prefix(&prefix1) {
                                    pending.push(idx_str.to_string());
                                }
                            } else if item.starts_with(&prefix2) {
                                if let Some(idx_str) = item.strip_prefix(&prefix2) {
                                    pending.push(idx_str.to_string());
                                }
                            }
                        }
                        for idx_str in pending {
                            known_strings.insert(format!("tuple_elem_str:{}:{}", name, idx_str));
                        }
                    }
                } else if let Expr::Array(elems) = value {
                    for (i, elem) in elems.iter().enumerate() {
                        if expr_is_definitely_string(elem, known_strings) {
                            known_strings.insert(format!("tuple_elem_str:{}:{}", name, i));
                        }
                    }
                }
                if let Some(ann) = type_ann {
                    let ann = ann.trim();
                    if ann.starts_with('(') && ann.ends_with(')') {
                        for (i, ty) in ann[1..ann.len() - 1].split(',').enumerate() {
                            let ty = ty.trim();
                            if ty == "string" || ty == "str" {
                                known_strings.insert(format!("tuple_elem_str:{}:{}", name, i));
                            }
                        }
                    }
                }
                if let Expr::Map(entries) = value {
                    for (k, v) in entries {
                        if expr_is_definitely_string(v, known_strings) {
                            if let Expr::String(field) = k {
                                known_strings.insert(format!("map_field_str:{}", field));
                                known_strings.insert(format!("map_str:{}.{}", name, field));
                                // Presence summary for the map-field
                                // scans (exact: set alongside).
                                known_strings.insert(format!("map_has_str:{}", name));
                            }
                        } else {
                            // Any non-string value voids the whole-map string
                            // claim (alya-lang/alya#39): loop values fall back
                            // to Number + runtime dispatch instead of `%s` on
                            // non-strings. Per-field markers stay (field reads
                            // are still precise); the veto only gates the
                            // map-wide aggregation in the for-loop consumer.
                            known_strings.insert(format!("map_nonstr:{}", name));
                        }
                    }
                }
                if let Expr::StructInit {
                    name: sname,
                    fields,
                } = value
                {
                    for (fname, fval) in fields {
                        if expr_is_definitely_string(fval, known_strings) {
                            if !struct_field_markers_mixed(known_strings, sname, fname) {
                                known_strings
                                    .insert(format!("struct_field_str:{}.{}", sname, fname));
                                if !conflicts.contains(fname) {
                                    known_strings.insert(format!("struct_field_str:{}", fname));
                                }
                            }
                            known_strings.insert(format!("{}.{}", name, fname));
                        }
                    }
                }
                if let Expr::Call { name: cname, args } = value {
                    if let Some(fnames) = struct_defs.get(cname) {
                        for (i, arg) in args.iter().enumerate() {
                            if expr_is_definitely_string(arg, known_strings) {
                                if let Some(fname) = fnames.get(i) {
                                    if !struct_field_markers_mixed(known_strings, cname, fname) {
                                        known_strings.insert(format!(
                                            "struct_field_str:{}.{}",
                                            cname, fname
                                        ));
                                        if !conflicts.contains(fname) {
                                            known_strings
                                                .insert(format!("struct_field_str:{}", fname));
                                        }
                                    }
                                    known_strings.insert(format!("{}.{}", name, fname));
                                }
                            }
                        }
                    }
                }
            }
            Stmt::Assign { name, value, .. } => {
                scan_expr_for_strings(value, struct_defs, known_strings, conflicts);
                if expr_is_definitely_string(value, known_strings) {
                    known_strings.insert(name.clone());
                }
                if expr_is_string_array(value, known_strings) {
                    known_strings.insert(format!("arr_is_str:{}", name));
                }
                if let Expr::Identifier(id) = value {
                    let bare_id = id.rsplit("::").next().unwrap_or(id.as_str());
                    let bare_id = bare_id.rsplit("__").next().unwrap_or(bare_id);
                    if known_strings.contains(&format!("fn_ret_str:{}", id))
                        // #101: qualified-spelled references consult exact only.
                        || (is_simple_name(id)
                            && known_strings.contains(&format!("fn_ret_str:{}", bare_id)))
                    {
                        known_strings.insert(format!("fn_ret_str:{}", name));
                    }
                }
                if let Expr::Call { name: cname, .. } = value {
                    let bare = cname.rsplit("::").next().unwrap_or(cname.as_str());
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    let prefix1 = format!("fn_ret_tuple_str:{}:", cname);
                    let prefix2 = format!("fn_ret_tuple_str:{}:", bare);
                    // Presence summaries (exact: set alongside the marker
                    // inserts below): skip the full-set scan unless a
                    // matching marker exists.
                    if known_strings.contains(&format!("fn_ret_tuple:{}", cname))
                        || known_strings.contains(&format!("fn_ret_tuple:{}", bare))
                    {
                        let mut pending: Vec<String> = Vec::new();
                        for item in known_strings.iter() {
                            if item.starts_with(&prefix1) {
                                if let Some(idx_str) = item.strip_prefix(&prefix1) {
                                    pending.push(idx_str.to_string());
                                }
                            } else if item.starts_with(&prefix2) {
                                if let Some(idx_str) = item.strip_prefix(&prefix2) {
                                    pending.push(idx_str.to_string());
                                }
                            }
                        }
                        for idx_str in pending {
                            known_strings.insert(format!("tuple_elem_str:{}:{}", name, idx_str));
                        }
                    }
                } else if let Expr::Array(elems) = value {
                    for (i, elem) in elems.iter().enumerate() {
                        if expr_is_definitely_string(elem, known_strings) {
                            known_strings.insert(format!("tuple_elem_str:{}:{}", name, i));
                        }
                    }
                }
                if let Expr::Map(entries) = value {
                    for (k, v) in entries {
                        if expr_is_definitely_string(v, known_strings) {
                            if let Expr::String(field) = k {
                                known_strings.insert(format!("map_field_str:{}", field));
                                known_strings.insert(format!("map_str:{}.{}", name, field));
                                // Presence summary for the map-field
                                // scans (exact: set alongside).
                                known_strings.insert(format!("map_has_str:{}", name));
                            }
                        } else {
                            // Any non-string value voids the whole-map string
                            // claim (alya-lang/alya#39): loop values fall back
                            // to Number + runtime dispatch instead of `%s` on
                            // non-strings. Per-field markers stay (field reads
                            // are still precise); the veto only gates the
                            // map-wide aggregation in the for-loop consumer.
                            known_strings.insert(format!("map_nonstr:{}", name));
                        }
                    }
                }
                if let Expr::StructInit {
                    name: sname,
                    fields,
                } = value
                {
                    for (fname, fval) in fields {
                        if expr_is_definitely_string(fval, known_strings) {
                            if !struct_field_markers_mixed(known_strings, sname, fname) {
                                known_strings
                                    .insert(format!("struct_field_str:{}.{}", sname, fname));
                                if !conflicts.contains(fname) {
                                    known_strings.insert(format!("struct_field_str:{}", fname));
                                }
                            }
                            known_strings.insert(format!("{}.{}", name, fname));
                        }
                    }
                }
                if let Expr::Call { name: cname, args } = value {
                    if let Some(fnames) = struct_defs.get(cname) {
                        for (i, arg) in args.iter().enumerate() {
                            if expr_is_definitely_string(arg, known_strings) {
                                if let Some(fname) = fnames.get(i) {
                                    if !struct_field_markers_mixed(known_strings, cname, fname) {
                                        known_strings.insert(format!(
                                            "struct_field_str:{}.{}",
                                            cname, fname
                                        ));
                                        if !conflicts.contains(fname) {
                                            known_strings
                                                .insert(format!("struct_field_str:{}", fname));
                                        }
                                    }
                                    known_strings.insert(format!("{}.{}", name, fname));
                                }
                            }
                        }
                    }
                }
            }
            Stmt::Say(expr) | Stmt::Expr(expr) => {
                scan_expr_for_strings(expr, struct_defs, known_strings, conflicts);
            }
            Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) => {
                scan_expr_for_strings(expr, struct_defs, known_strings, conflicts);
            }
            Stmt::FieldAssign {
                object,
                field,
                value,
            } => {
                scan_expr_for_strings(value, struct_defs, known_strings, conflicts);
                if expr_is_definitely_string(value, known_strings) {
                    if !conflicts.contains(field)
                        && !known_strings.contains(&format!("struct_field_mixed:{}", field))
                    {
                        known_strings.insert(format!("struct_field_str:{}", field));
                    }
                    if let Expr::Identifier(obj_name) = object {
                        known_strings.insert(format!("{}.{}", obj_name, field));
                    }
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_var,
                catch_block,
                finally_block,
            } => {
                if let Some(err_var) = catch_var {
                    known_strings.insert(err_var.clone());
                }
                collect_string_vars_from_stmts(try_block, struct_defs, known_strings, conflicts);
                collect_string_vars_from_stmts(catch_block, struct_defs, known_strings, conflicts);
                if let Some(finally_block) = finally_block {
                    collect_string_vars_from_stmts(
                        finally_block,
                        struct_defs,
                        known_strings,
                        conflicts,
                    );
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                scan_expr_for_strings(condition, struct_defs, known_strings, conflicts);
                collect_string_vars_from_stmts(then_block, struct_defs, known_strings, conflicts);
                if let Some(else_stmts) = else_block {
                    collect_string_vars_from_stmts(
                        else_stmts,
                        struct_defs,
                        known_strings,
                        conflicts,
                    );
                }
            }
            Stmt::While { condition, body } => {
                scan_expr_for_strings(condition, struct_defs, known_strings, conflicts);
                collect_string_vars_from_stmts(body, struct_defs, known_strings, conflicts);
            }
            Stmt::Repeat { body } => {
                collect_string_vars_from_stmts(body, struct_defs, known_strings, conflicts);
            }
            Stmt::For {
                start, end, body, ..
            } => {
                scan_expr_for_strings(start, struct_defs, known_strings, conflicts);
                scan_expr_for_strings(end, struct_defs, known_strings, conflicts);
                collect_string_vars_from_stmts(body, struct_defs, known_strings, conflicts);
            }
            Stmt::ForEach {
                var,
                value_var,
                iterable,
                body,
            } => {
                scan_expr_for_strings(iterable, struct_defs, known_strings, conflicts);
                // Transparent wrapper: classify the inner expression.
                let iterable_inner: &Expr = match iterable {
                    Expr::ForceUnwrap(inner) => inner,
                    other => other,
                };
                let is_map = match iterable_inner {
                    Expr::Map(_) => true,
                    // Presence-gated (exact: summaries are inserted
                    // alongside the markers): skip the full-set scan
                    // unless this map has field markers. The map_map
                    // half has no writers in this set; keep it verbatim.
                    Expr::Identifier(name) => {
                        known_strings.contains(&format!("map_has_str:{}", name))
                            || known_strings
                                .iter()
                                .any(|k| k.starts_with(&format!("map_map:{}.", name)))
                    }
                    _ => false,
                };
                if let Some(v) = value_var {
                    if is_map {
                        known_strings.insert(var.clone());
                        let val_is_str = match iterable_inner {
                            Expr::Map(entries) => entries
                                .iter()
                                .any(|(_, val)| expr_is_definitely_string(val, known_strings)),
                            Expr::Identifier(name) => {
                                known_strings.contains(&format!("map_has_str:{}", name))
                            }
                            _ => false,
                        };
                        if val_is_str {
                            known_strings.insert(v.clone());
                        }
                    } else if expr_is_string_array(iterable, known_strings) {
                        known_strings.insert(v.clone());
                    }
                } else if is_map || expr_is_string_array(iterable, known_strings) {
                    known_strings.insert(var.clone());
                }
                collect_string_vars_from_stmts(body, struct_defs, known_strings, conflicts);
            }
            Stmt::Function {
                name, params, body, ..
            } => {
                let bare = name.rsplit("::").next().unwrap_or(name.as_str());
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                // Transitive vetoes first: forwarding a proven-dynamic
                // param into a helper vetoes the helper (see above).
                record_forwarding_param_vetoes(name, params, body, known_strings);
                // Return-position vetoes: a literal non-string return
                // proves the function dynamic (see above).
                record_return_nonstr_vetoes(name, body, known_strings);
                let mut fn_locals = known_strings.clone();
                for (idx, param) in params.iter().enumerate() {
                    // Same veto as infer_param_is_string_with: a literal
                    // non-string call proves the parameter is dynamic.
                    // #101: a bare veto from an unrelated same-bare
                    // function must not block this function's marking.
                    let vetoed = known_strings
                        .contains(&format!("fn_param_nonstr:{}:{}", name, idx))
                        || (is_simple_name(name)
                            && known_strings
                                .contains(&format!("fn_param_nonstr:{}:{}", bare, idx)));
                    // #101: qualified-spelled functions consult exact
                    // markers only.
                    let bare_ok = is_simple_name(name);
                    if !vetoed
                        && (known_strings.contains(&format!("fn_param_str:{}:{}", name, idx))
                            || (bare_ok
                                && known_strings
                                    .contains(&format!("fn_param_str:{}:{}", bare, idx))))
                    {
                        fn_locals.insert(param.clone());
                    }
                    if known_strings.contains(&format!("fn_param_str_arr:{}:{}", name, idx))
                        || (bare_ok
                            && known_strings
                                .contains(&format!("fn_param_str_arr:{}:{}", bare, idx)))
                    {
                        fn_locals.insert(format!("arr_is_str:{}", param));
                    }
                }
                let prefix1 = format!("fn_local_str_arr:{}:", name);
                let prefix2 = format!("fn_local_str_arr:{}:", bare);
                // Presence-gated: skip the full-set scan unless a
                // matching marker exists (exact: summaries are inserted
                // alongside the markers above).
                if known_strings.contains(&format!("has_local_str_arr:{}", name))
                    || known_strings.contains(&format!("has_local_str_arr:{}", bare))
                {
                    for item in known_strings.iter() {
                        if let Some(var_name) = item.strip_prefix(&prefix1) {
                            fn_locals.insert(format!("arr_is_str:{}", var_name));
                        } else if may_record_bare!(known_strings, name, bare) {
                            if let Some(var_name) = item.strip_prefix(&prefix2) {
                                fn_locals.insert(format!("arr_is_str:{}", var_name));
                            }
                        }
                    }
                }
                collect_string_vars_from_stmts(body, struct_defs, &mut fn_locals, conflicts);
                // Existential `fn_ret_str` positives must be gated by return
                // vetoes: one literal non-string return proves the function
                // dynamic, so trusting the positive would fold `is string`
                // to constant-true and corrupt heap results (`str_store`).
                let ret_vetoed = known_strings.contains(&format!("fn_ret_nonstr:{}", name))
                    // #101: a bare veto from an unrelated same-bare
                    // function must not block this function's marking.
                    || (is_simple_name(name)
                        && known_strings.contains(&format!("fn_ret_nonstr:{}", bare)));
                if !ret_vetoed
                    && stmts_return_string(body, &fn_locals)
                    && !matches!(
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
                            | "json_parse_array"
                            | "parse_array"
                            | "json_parse_object"
                            | "parse_object"
                            | "json_parse"
                            | "parse"
                            | "_json_parse_val"
                    )
                {
                    known_strings.insert(format!("fn_ret_str:{}", name));
                    // #101: bare markers need a single owner.
                    if may_record_bare!(known_strings, name, bare) {
                        known_strings.insert(format!("fn_ret_str:{}", bare));
                    }
                }
                collect_tuple_returns_string(body, &fn_locals, name, known_strings);
                if may_record_bare!(known_strings, name, bare) {
                    collect_tuple_returns_string(body, &fn_locals, bare, known_strings);
                }
                collect_function_returns_string_array(body, &fn_locals, name, known_strings);
                if may_record_bare!(known_strings, name, bare) {
                    collect_function_returns_string_array(body, &fn_locals, bare, known_strings);
                }
                for item in &fn_locals {
                    if let Some(var_name) = item.strip_prefix("arr_is_str:") {
                        known_strings.insert(format!("fn_local_str_arr:{}:{}", name, var_name));
                        // Presence summary for the scan below.
                        known_strings.insert(format!("has_local_str_arr:{}", name));
                        if may_record_bare!(known_strings, name, bare) {
                            known_strings.insert(format!("fn_local_str_arr:{}:{}", bare, var_name));
                            known_strings.insert(format!("has_local_str_arr:{}", bare));
                        }
                    }
                    // Skip re-exporting markers already global
                    // (monotonic: present items re-insert identically).
                    if known_strings.contains(item) {
                        continue;
                    }
                    if item.starts_with("fn_ret_str:")
                        || item.starts_with("fn_ret_str_arr:")
                        || item.starts_with("fn_ret_tuple_str:")
                        || item.starts_with("tuple_elem_str:")
                        || item.starts_with("map_field_str:")
                        || item.starts_with("map_str:")
                        || item.starts_with("struct_field_str:")
                        || item.starts_with("fn_param_str:")
                        || item.starts_with("fn_param_nonstr:")
                        || item.starts_with("fn_ret_nonstr:")
                        || item.starts_with("fn_param_str_arr:")
                    {
                        known_strings.insert(item.clone());
                    }
                }
            }
            Stmt::IndexAssign {
                array,
                index,
                value,
            } => {
                scan_expr_for_strings(index, struct_defs, known_strings, conflicts);
                scan_expr_for_strings(value, struct_defs, known_strings, conflicts);
                if expr_is_definitely_string(value, known_strings) {
                    if let Expr::String(field) = index {
                        known_strings.insert(format!("map_field_str:{}", field));
                    }
                    if let (Expr::Identifier(map_name), Expr::String(field)) = (array, index) {
                        known_strings.insert(format!("map_str:{}.{}", map_name, field));
                        // Presence summary for the map-field scans
                        // (exact: set alongside).
                        known_strings.insert(format!("map_has_str:{}", map_name));
                    }
                } else if let (Expr::Identifier(map_name), Expr::String(_)) = (array, index) {
                    // Non-string write voids the whole-map string claim
                    // (alya-lang/alya#39); see the Map-literal veto above.
                    known_strings.insert(format!("map_nonstr:{}", map_name));
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_string_vars_from_stmts(
                    std::slice::from_ref(inner),
                    struct_defs,
                    known_strings,
                    conflicts,
                );
            }
            _ => {}
        }
    }
}

pub fn collect_known_string_vars(program: &Program) -> HashSet<String> {
    let call_index = CallIndex::build(&program.statements);
    collect_known_string_vars_with_index(program, &call_index)
}

/// Bare field names whose `string` type is contradicted by another struct's
/// explicit non-string declaration (e.g. `Url.port: string` vs
/// `Srv.port: int`).
///
/// Bare `struct_field_str:{field}` markers for such fields are unsound: any
/// read of an unrelated struct's same-named non-string field (e.g.
/// `srv.port`) would be misclassified as a string, which once miscompiled
/// `is null` into `strcmp` on an integer and segfaulted `say`. Qualified
/// markers (`Struct.field`, `var.field`) stay precise and are unaffected.
fn conflicting_string_fields(program: &Program) -> HashSet<String> {
    fn walk(stmts: &[Stmt], kinds: &mut HashMap<String, (bool, bool)>) {
        for s in stmts {
            match s {
                Stmt::StructDef {
                    fields,
                    field_types,
                    ..
                } => {
                    for (f, ft) in fields.iter().zip(field_types.iter()) {
                        let e = kinds.entry(f.clone()).or_insert((false, false));
                        match ft.as_deref() {
                            Some("string") | Some("str") => e.0 = true,
                            None | Some("any") | Some("auto") => {}
                            Some(_) => e.1 = true,
                        }
                    }
                }
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    walk(then_block, kinds);
                    if let Some(eb) = else_block {
                        walk(eb, kinds);
                    }
                }
                Stmt::While { body, .. }
                | Stmt::Repeat { body }
                | Stmt::For { body, .. }
                | Stmt::ForEach { body, .. }
                | Stmt::Function { body, .. } => walk(body, kinds),
                Stmt::TryCatch {
                    try_block,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    walk(try_block, kinds);
                    walk(catch_block, kinds);
                    if let Some(fb) = finally_block {
                        walk(fb, kinds);
                    }
                }
                Stmt::Pub(inner) | Stmt::Defer(inner) => {
                    walk(std::slice::from_ref(inner), kinds);
                }
                _ => {}
            }
        }
    }
    let mut kinds: HashMap<String, (bool, bool)> = HashMap::new();
    walk(&program.statements, &mut kinds);
    kinds
        .into_iter()
        .filter(|(_, (has_str, has_other))| *has_str && *has_other)
        .map(|(f, _)| f)
        .collect()
}

/// Literal value kinds observed at struct construction sites, used to
/// suppress unsound global field markers. A field constructed with two
/// different literal kinds (e.g. `Box{value: 1}` and `Box{value: "s"}`)
/// cannot be served by one static marker: the losing instance's reads
/// miscompile (`%s` on an int segfaults; `%g` on int bits prints
/// garbage). Kinds: 0 = string, 1 = float, 2 = other literal,
/// 3 = dynamic (non-literal value: identifier, call result, field/index
/// read, ...). Dynamic values prove nothing about the field's type, so a
/// dynamic observation mixed with any literal kind poisons the static
/// marker exactly like two conflicting literals (alya-lang/alya#113:
/// `HttpRoute{action: handle_nop}` ignored + `RouteMatch{action:
/// "not_found"}` recorded poisoned bare `struct_field_str:action`, and
/// the function-valued read `rt.routes[0].action` miscompiled to a
/// string load and segfaulted on call).
fn struct_field_lit_kind(expr: &Expr) -> Option<u8> {
    match expr {
        Expr::String(_) | Expr::InterpolatedString(_) => Some(0),
        Expr::Float(_) => Some(1),
        Expr::Number(_) => Some(2),
        Expr::Null | Expr::Array(_) | Expr::Map(_) | Expr::StructInit { .. } => Some(2),
        _ => None,
    }
}

/// Kind bit for dynamic (non-literal) struct field values. Any field
/// mixing this with a literal kind (or with literal kinds across
/// same-named fields of other structs) earns a `struct_field_mixed`
/// sentinel, suppressing its global markers.
const STRUCT_FIELD_DYN_KIND: u8 = 3;

#[derive(Default)]
struct StructFieldLitKinds {
    /// (struct, field) -> bitmask of observed kinds (literal kinds 0-2
    /// plus dynamic kind 3). Structs keyed
    /// both as-written and bare to cover read-side lookup variations.
    qualified: HashMap<(String, String), u8>,
    /// field -> bitmask across all structs plus unattributable field
    /// assigns. Guards the shared bare markers.
    bare: HashMap<String, u8>,
}

fn observe_struct_field_kind(kinds: &mut StructFieldLitKinds, sname: &str, fname: &str, kind: u8) {
    let bit = 1u8 << kind;
    let bare_s = sname.rsplit("::").next().unwrap_or(sname);
    let bare_s = bare_s.rsplit("__").next().unwrap_or(bare_s);
    *kinds
        .qualified
        .entry((sname.to_string(), fname.to_string()))
        .or_default() |= bit;
    if bare_s != sname {
        *kinds
            .qualified
            .entry((bare_s.to_string(), fname.to_string()))
            .or_default() |= bit;
    }
    *kinds.bare.entry(fname.to_string()).or_default() |= bit;
}

fn collect_struct_kinds_from_expr(
    expr: &Expr,
    struct_defs: &HashMap<String, Vec<String>>,
    kinds: &mut StructFieldLitKinds,
) {
    match expr {
        Expr::Call { name, args } => {
            if let Some(fields) = struct_defs.get(name) {
                for (i, arg) in args.iter().enumerate() {
                    if let Some(fname) = fields.get(i) {
                        // Non-literal args are dynamic observations, not
                        // absences (alya-lang/alya#113).
                        let k = struct_field_lit_kind(arg).unwrap_or(STRUCT_FIELD_DYN_KIND);
                        observe_struct_field_kind(kinds, name, fname, k);
                    }
                }
            }
            for arg in args {
                collect_struct_kinds_from_expr(arg, struct_defs, kinds);
            }
        }
        Expr::StructInit { name, fields } => {
            for (fname, fval) in fields {
                // Non-literal values are dynamic observations, not
                // absences (alya-lang/alya#113).
                let k = struct_field_lit_kind(fval).unwrap_or(STRUCT_FIELD_DYN_KIND);
                observe_struct_field_kind(kinds, name, fname, k);
                collect_struct_kinds_from_expr(fval, struct_defs, kinds);
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_struct_kinds_from_expr(left, struct_defs, kinds);
            collect_struct_kinds_from_expr(right, struct_defs, kinds);
        }
        Expr::Unary { expr, .. } => {
            collect_struct_kinds_from_expr(expr, struct_defs, kinds);
        }
        Expr::ForceUnwrap(inner) => {
            collect_struct_kinds_from_expr(inner, struct_defs, kinds);
        }
        Expr::Array(elems) | Expr::InterpolatedString(elems) => {
            for elem in elems {
                collect_struct_kinds_from_expr(elem, struct_defs, kinds);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            collect_struct_kinds_from_expr(array, struct_defs, kinds);
            collect_struct_kinds_from_expr(index, struct_defs, kinds);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            collect_struct_kinds_from_expr(object, struct_defs, kinds);
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                collect_struct_kinds_from_expr(k, struct_defs, kinds);
                collect_struct_kinds_from_expr(v, struct_defs, kinds);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_struct_kinds_from_expr(condition, struct_defs, kinds);
            collect_struct_kinds_from_expr(then_branch, struct_defs, kinds);
            collect_struct_kinds_from_expr(else_branch, struct_defs, kinds);
        }
        Expr::NullCoalesce { value, default } => {
            collect_struct_kinds_from_expr(value, struct_defs, kinds);
            collect_struct_kinds_from_expr(default, struct_defs, kinds);
        }
        Expr::OptionalCall { args, .. } => {
            for arg in args {
                collect_struct_kinds_from_expr(arg, struct_defs, kinds);
            }
        }
        Expr::TypeCheck { expr, .. } | Expr::Cast { expr, .. } => {
            collect_struct_kinds_from_expr(expr, struct_defs, kinds);
        }
        _ => {}
    }
}

fn collect_struct_field_lit_kinds(
    stmts: &[Stmt],
    struct_defs: &HashMap<String, Vec<String>>,
    kinds: &mut StructFieldLitKinds,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let { value, .. } | Stmt::Const { value, .. } | Stmt::Assign { value, .. } => {
                collect_struct_kinds_from_expr(value, struct_defs, kinds);
            }
            Stmt::Say(expr) | Stmt::Expr(expr) => {
                collect_struct_kinds_from_expr(expr, struct_defs, kinds);
            }
            Stmt::Return(expr) | Stmt::Throw(expr) => {
                if let Some(expr) = expr {
                    collect_struct_kinds_from_expr(expr, struct_defs, kinds);
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_struct_kinds_from_expr(condition, struct_defs, kinds);
                collect_struct_field_lit_kinds(then_block, struct_defs, kinds);
                if let Some(eb) = else_block {
                    collect_struct_field_lit_kinds(eb, struct_defs, kinds);
                }
            }
            Stmt::While { condition, body } => {
                collect_struct_kinds_from_expr(condition, struct_defs, kinds);
                collect_struct_field_lit_kinds(body, struct_defs, kinds);
            }
            Stmt::Repeat { body } => {
                collect_struct_field_lit_kinds(body, struct_defs, kinds);
            }
            Stmt::For {
                start, end, body, ..
            } => {
                collect_struct_kinds_from_expr(start, struct_defs, kinds);
                collect_struct_kinds_from_expr(end, struct_defs, kinds);
                collect_struct_field_lit_kinds(body, struct_defs, kinds);
            }
            Stmt::ForEach { iterable, body, .. } => {
                collect_struct_kinds_from_expr(iterable, struct_defs, kinds);
                collect_struct_field_lit_kinds(body, struct_defs, kinds);
            }
            Stmt::Function { body, defaults, .. } => {
                for d in defaults.iter().flatten() {
                    collect_struct_kinds_from_expr(d, struct_defs, kinds);
                }
                collect_struct_field_lit_kinds(body, struct_defs, kinds);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_struct_field_lit_kinds(try_block, struct_defs, kinds);
                collect_struct_field_lit_kinds(catch_block, struct_defs, kinds);
                if let Some(fb) = finally_block {
                    collect_struct_field_lit_kinds(fb, struct_defs, kinds);
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_struct_field_lit_kinds(std::slice::from_ref(inner), struct_defs, kinds);
            }
            Stmt::FieldAssign {
                object,
                field,
                value,
            } => {
                collect_struct_kinds_from_expr(object, struct_defs, kinds);
                // Non-literal values are dynamic observations, not absences
                // (alya-lang/alya#113).
                let k = struct_field_lit_kind(value).unwrap_or(STRUCT_FIELD_DYN_KIND);
                *kinds.bare.entry(field.clone()).or_default() |= 1u8 << k;
                collect_struct_kinds_from_expr(value, struct_defs, kinds);
            }
            Stmt::IndexAssign {
                array,
                index,
                value,
            } => {
                collect_struct_kinds_from_expr(array, struct_defs, kinds);
                collect_struct_kinds_from_expr(index, struct_defs, kinds);
                collect_struct_kinds_from_expr(value, struct_defs, kinds);
            }
            Stmt::StructDef {
                name,
                fields,
                defaults,
                ..
            } => {
                // Declared defaults are possible values when callers omit args.
                for (f, d) in fields.iter().zip(defaults.iter()) {
                    if let Some(d) = d {
                        // Non-literal defaults are dynamic observations, not
                        // absences (alya-lang/alya#113).
                        let k = struct_field_lit_kind(d).unwrap_or(STRUCT_FIELD_DYN_KIND);
                        observe_struct_field_kind(kinds, name, f, k);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Sentinel keys marking struct fields constructed with mixed literal
/// kinds. Both pre-scan inserts and codegen-time marker emission skip
/// global markers for these fields; reads fall back to runtime
/// classification (correct for ints/strings) plus per-variable keys.
fn insert_struct_field_mixed_sentinels(
    kinds: &StructFieldLitKinds,
    known_strings: &mut HashSet<String>,
) {
    fn is_mixed(mask: u8) -> bool {
        mask & mask.wrapping_sub(1) != 0
    }
    for ((s, f), mask) in &kinds.qualified {
        if is_mixed(*mask) {
            known_strings.insert(format!("struct_field_mixed:{}.{}", s, f));
        }
    }
    for (f, mask) in &kinds.bare {
        if is_mixed(*mask) {
            known_strings.insert(format!("struct_field_mixed:{}", f));
        }
    }
}

/// True when the global field markers for `(sname, fname)` are unusable:
/// mixed literal kinds were observed, so no single static type serves
/// every instance. Checks qualified (as-written + bare struct) and bare
/// field sentinels.
fn struct_field_markers_mixed(known: &HashSet<String>, sname: &str, fname: &str) -> bool {
    let bare_s = sname.rsplit("::").next().unwrap_or(sname);
    let bare_s = bare_s.rsplit("__").next().unwrap_or(bare_s);
    known.contains(&format!("struct_field_mixed:{}.{}", sname, fname))
        || known.contains(&format!("struct_field_mixed:{}.{}", bare_s, fname))
        || known.contains(&format!("struct_field_mixed:{}", fname))
}

pub fn collect_known_string_vars_with_index(
    program: &Program,
    call_index: &CallIndex,
) -> HashSet<String> {
    let mut known_strings = HashSet::new();
    // #101: ambiguity sentinels first; collided bare keys stay unrecorded.
    seed_ambiguity_markers(program, &mut known_strings);
    // Fields whose `string` type is contradicted by another struct's explicit
    // non-string declaration (e.g. `Url.port: string` vs `Srv.port: int`).
    // Bare `struct_field_str:{field}` markers for such fields are unsound and
    // are skipped; qualified markers stay precise.
    let conflicts = conflicting_string_fields(program);
    for stmt in &program.statements {
        let stmt = stmt.inner_stmt();
        if let Stmt::ExternBlock { functions, .. } = stmt {
            for f in functions {
                if let Some(ret) = &f.return_type {
                    if ret == "str" || ret == "string" {
                        known_strings.insert(format!("fn_ret_str:{}", f.name));
                    }
                }
            }
        } else if let Stmt::Function {
            name,
            param_types,
            return_type,
            ..
        } = stmt
        {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            // #101: bare markers need a single owner: simple spellings
            // always (the exact key IS the bare key), qualified
            // spellings only when no other function shares the bare.
            let record_bare = may_record_bare!(known_strings, name, bare);
            if let Some(ret) = return_type {
                if ret == "str" || ret == "string" {
                    known_strings.insert(format!("fn_ret_str:{}", name));
                    if record_bare {
                        known_strings.insert(format!("fn_ret_str:{}", bare));
                    }
                } else if ret == "str[]" || ret == "string[]" {
                    known_strings.insert(format!("fn_ret_str_arr:{}", name));
                    if record_bare {
                        known_strings.insert(format!("fn_ret_str_arr:{}", bare));
                    }
                }
            }
            for (idx, p_type) in param_types.iter().enumerate() {
                if let Some(pt) = p_type {
                    if pt == "str" || pt == "string" {
                        known_strings.insert(format!("fn_param_str:{}:{}", name, idx));
                        if record_bare {
                            known_strings.insert(format!("fn_param_str:{}:{}", bare, idx));
                        }
                    } else if pt == "str[]" || pt == "string[]" {
                        known_strings.insert(format!("fn_param_str_arr:{}:{}", name, idx));
                        if record_bare {
                            known_strings.insert(format!("fn_param_str_arr:{}:{}", bare, idx));
                        }
                    }
                }
            }
        } else if let Stmt::StructDef {
            name,
            fields,
            field_types,
            ..
        } = stmt
        {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            for (f, ft) in fields.iter().zip(field_types.iter()) {
                if let Some(t) = ft {
                    if t == "string" || t == "str" {
                        known_strings.insert(format!("struct_field_str:{}.{}", name, f));
                        known_strings.insert(format!("struct_field_str:{}.{}", bare, f));
                        if !conflicts.contains(f) {
                            known_strings.insert(format!("struct_field_str:{}", f));
                        }
                    } else if t == "string[]" || t == "str[]" {
                        known_strings.insert(format!("struct_field_arr_str:{}.{}", name, f));
                        known_strings.insert(format!("struct_field_arr_str:{}.{}", bare, f));
                        known_strings.insert(format!("struct_field_arr_str:{}", f));
                    }
                }
            }
        }
    }
    let mut funcs = Vec::new();
    collect_function_defs(&program.statements, &mut funcs);
    let mut struct_defs = HashMap::new();
    collect_struct_defs(&program.statements, &mut struct_defs);
    // Mixed literal kinds per struct field (e.g. `Box{value: 1}` and
    // `Box{value: "s"}`): no single static marker serves every instance,
    // so global markers for such fields are suppressed everywhere below.
    // Sentinels live in known_strings so function-body scans (which clone
    // the set) and codegen (via ctx seeding) see them uniformly.
    let mut field_kinds = StructFieldLitKinds::default();
    collect_struct_field_lit_kinds(&program.statements, &struct_defs, &mut field_kinds);
    insert_struct_field_mixed_sentinels(&field_kinds, &mut known_strings);
    // Negative evidence first (issue #45): a literal non-string call arg
    // proves the parameter is dynamic no matter where the call sits, but
    // the fixpoint loop below scans statements in order — a positive
    // derived early (e.g. from a same-bare forwarding call) lands before
    // its veto is seen, persists monotonically, and the veto (recorded
    // under the call's bare name) never matches the positive's resolved
    // name at consumption. Veto insertion is purely syntactic, hence
    // order-independent, so seeding all vetoes up front is semantics-
    // preserving and closes the ordering hole. Only `fn_param_nonstr:*`
    // and `fn_ret_nonstr:*` markers are merged; positives from the
    // pre-scan are discarded.
    {
        let mut pre = HashSet::new();
        collect_string_vars_from_stmts(&program.statements, &struct_defs, &mut pre, &conflicts);
        for m in pre {
            if m.starts_with("fn_param_nonstr:") || m.starts_with("fn_ret_nonstr:") {
                known_strings.insert(m);
            }
        }
    }
    // Return-veto closure (alya-lang/alya#107): transitive vetoes
    // (`return F(...)` where F is already vetoed) propagate one call
    // level per pass, so close them before any positive is derived —
    // otherwise a positive recorded in an early round outlives the
    // veto that should have blocked it. Cheap syntactic walk.
    {
        let mut funcs = Vec::new();
        collect_function_defs(&program.statements, &mut funcs);
        for _ in 0..10 {
            let prev_len = known_strings.len();
            for (name, _, _, body) in &funcs {
                record_return_nonstr_vetoes(name, body, &mut known_strings);
            }
            if known_strings.len() == prev_len {
                break;
            }
        }
    }
    for _ in 0..5 {
        let prev_len = known_strings.len();
        collect_string_vars_from_stmts(
            &program.statements,
            &struct_defs,
            &mut known_strings,
            &conflicts,
        );
        for (name, params, _, _) in &funcs {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            for (idx, _param) in params.iter().enumerate() {
                // Universal derivation is keyed on the exact name: a bare
                // marker from the per-call rule (or another same-bare
                // function) must not suppress it (alya-lang/alya#105 —
                // bare-spelled UFCS method calls left qualified
                // definitions without exact param markers).
                if !known_strings.contains(&format!("fn_param_str_arr:{}:{}", name, idx)) {
                    let mut call_args = Vec::new();
                    call_index.collect_all_call_args(name, bare, idx, &mut call_args);
                    if !call_args.is_empty()
                        && call_args
                            .iter()
                            .all(|arg| expr_is_string_array(arg, &known_strings))
                    {
                        known_strings.insert(format!("fn_param_str_arr:{}:{}", name, idx));
                        // #101: bare markers need a single owner.
                        if may_record_bare!(known_strings, name, bare) {
                            known_strings.insert(format!("fn_param_str_arr:{}:{}", bare, idx));
                        }
                    }
                }
                // Universal (ALL call sites), mirroring the float/array
                // param rules below and above: one non-string call proves
                // the parameter is dynamic, so an existential (ANY) rule
                // would fold `is string` to constant-true and miscompile
                // every other call (`%s` on an int segfaults).
                // Universal derivation is keyed on the exact name: a bare
                // marker from the per-call rule (or another same-bare
                // function) must not suppress it (alya-lang/alya#105).
                if !known_strings.contains(&format!("fn_param_str:{}:{}", name, idx)) {
                    let mut call_args = Vec::new();
                    call_index.collect_all_call_args(name, bare, idx, &mut call_args);
                    if !call_args.is_empty()
                        && call_args
                            .iter()
                            .all(|arg| expr_is_definitely_string(arg, &known_strings))
                    {
                        known_strings.insert(format!("fn_param_str:{}:{}", name, idx));
                        // #101: bare markers need a single owner.
                        if may_record_bare!(known_strings, name, bare) {
                            known_strings.insert(format!("fn_param_str:{}:{}", bare, idx));
                        }
                    }
                }
            }
        }
        if known_strings.len() == prev_len {
            break;
        }
    }
    known_strings
}

pub fn infer_param_is_string_with(
    func_name: &str,
    param_idx: usize,
    _program: &Program,
    known_strings: &HashSet<String>,
) -> bool {
    let bare = func_name.rsplit("::").next().unwrap_or(func_name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    // A literal non-string call vetoes the existential positive: the
    // parameter is dynamic and `is string` must discriminate at runtime.
    let vetoed = known_strings.contains(&format!("fn_param_nonstr:{}:{}", func_name, param_idx))
        // #101: a bare veto from an unrelated same-bare function must
        // not block this function's marking.
        || (is_simple_name(func_name)
            && known_strings.contains(&format!("fn_param_nonstr:{}:{}", bare, param_idx)));
    !vetoed
        && (known_strings.contains(&format!("fn_param_str:{}:{}", func_name, param_idx))
            // #101: qualified-spelled callees consult exact markers only.
            || (is_simple_name(func_name)
                && known_strings.contains(&format!("fn_param_str:{}:{}", bare, param_idx))))
}

pub fn infer_param_is_string_array_with(
    func_name: &str,
    param_idx: usize,
    _program: &Program,
    known_strings: &HashSet<String>,
) -> bool {
    let bare = func_name.rsplit("::").next().unwrap_or(func_name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    known_strings.contains(&format!("fn_param_str_arr:{}:{}", func_name, param_idx))
        // #101: qualified-spelled callees consult exact markers only.
        || (is_simple_name(func_name)
            && known_strings.contains(&format!("fn_param_str_arr:{}:{}", bare, param_idx)))
}

pub fn infer_param_is_string(func_name: &str, param_idx: usize, program: &Program) -> bool {
    let known_strings = collect_known_string_vars(program);
    infer_param_is_string_with(func_name, param_idx, program, &known_strings)
}

pub fn infer_param_is_string_array(func_name: &str, param_idx: usize, program: &Program) -> bool {
    let known_strings = collect_known_string_vars(program);
    infer_param_is_string_array_with(func_name, param_idx, program, &known_strings)
}
