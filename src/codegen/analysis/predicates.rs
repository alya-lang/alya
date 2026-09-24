use crate::ast::*;
use crate::codegen::context::VarType;
use std::collections::HashMap;

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
            vars.contains_key(&format!("fn_ret_str:{}", name))
                || vars.contains_key(&format!("fn_ret_str:{}", bare))
                || vars.keys().any(|k| {
                    k.starts_with("fn_ret_str:")
                        && (k.ends_with(&format!("__{}", bare))
                            || k.ends_with(&format!("::{}", bare)))
                })
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
                let key = format!("map_field_str:{}", field);
                if vars.contains_key(&key) {
                    return true;
                }
            }
            if let (Expr::Identifier(obj_name), Expr::String(field)) = (&**array, &**index) {
                let key = format!("map_str:{}.{}", obj_name, field);
                if vars.contains_key(&key) {
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
            if vars.contains_key(&global_field_key) {
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
                    | "json_parse_array"
                    | "parse_array"
            ) || (bare == "slice" && !args.is_empty() && is_array_expr(&args[0], vars))
                || vars.contains_key(&format!("fn_ret_arr:{}", name))
                || vars.contains_key(&format!("fn_ret_arr:{}", bare))
                || vars.contains_key(&format!("fn_ret_arr:{}", name.replace("::", "__")))
                || vars.contains_key(&format!("fn_ret_arr:{}", name.replace("__", "::")))
                || vars.contains_key(&format!("fn_ret_str_arr:{}", name))
                || vars.contains_key(&format!("fn_ret_str_arr:{}", bare))
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
            if vars.contains_key(&global_field_key) {
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
                    | "url_parse_query"
                    | "json_parse"
                    | "json_parse_object"
                    | "parse_object"
            ) || vars.contains_key(&format!("fn_ret_map:{}", name))
                || vars.contains_key(&format!("fn_ret_map:{}", bare))
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
            if vars.contains_key(&global_field_key) {
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

pub fn is_string_array(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Array(elems) => !elems.is_empty() && elems.iter().all(|e| is_string_expr(e, vars)),
        Expr::Identifier(name) => vars.contains_key(&format!("arr_is_str:{}", name)),
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
            vars.contains_key(&global_field_key)
        }
        Expr::Call { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            matches!(
                bare,
                "split"
                    | "args"
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
        Expr::Array(elems) => elems.first().is_some_and(|e| is_float_expr(e, vars)),
        Expr::Identifier(name) => vars.contains_key(&format!("arr_is_flt:{}", name)),
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
            vars.contains_key(&global_field_key)
        }
        Expr::ForceUnwrap(inner) => is_float_array(inner, vars),
        _ => false,
    }
}

pub fn is_float_expr(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Float(_) => true,
        Expr::Number(n) => n.fract() != 0.0,
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
            if vars.contains_key(&global_field_key) {
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
            is_float_array(array, vars)
        }
        Expr::Call { name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if bare == "get" && args.len() >= 3 && is_float_expr(&args[2], vars) {
                return true;
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
                    | "native_log"
                    | "native_log2"
                    | "native_log10"
                    | "native_exp"
                    | "native_sqrt"
                    | "native_ceil"
                    | "native_floor"
                    | "native_fmod"
                    | "asin"
                    | "acos"
                    | "atan"
                    | "atan2"
                    | "sinh"
                    | "cosh"
                    | "tanh"
                    | "log_n"
                    | "log2"
                    | "log10"
                    | "exp_f"
                    | "fmod"
                    | "sqrt_f"
            ) || vars.contains_key(&format!("fn_ret_flt:{}", name))
                || vars.contains_key(&format!("fn_ret_flt:{}", bare))
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

pub fn is_number_expr(expr: &Expr, vars: &HashMap<String, VarType>) -> bool {
    match expr {
        Expr::Number(_) => true,
        Expr::Identifier(name) => matches!(vars.get(name), Some(VarType::Number(_))),
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
            )
        }
        Expr::ForceUnwrap(inner) => is_number_expr(inner, vars),
        _ => false,
    }
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
