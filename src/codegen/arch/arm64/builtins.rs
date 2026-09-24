use super::emit_adrp_add;
use super::loads::{emit_arm64_load_x29_offset, emit_arm64_store_x29_offset};
use crate::ast::BinaryOp;
use crate::codegen::target::OperatingSystem;

pub fn emit_try_begin(out: &mut String, catch_label: &str, os: OperatingSystem) {
    emit_adrp_add(out, "x9", "alya_catch_idx", os);
    out.push_str("    ldr x10, [x9]\n");
    emit_adrp_add(out, "x11", catch_label, os);
    emit_adrp_add(out, "x12", "alya_catch_stack_handler", os);
    out.push_str("    str x11, [x12, x10, lsl #3]\n");
    out.push_str("    mov x13, sp\n");
    emit_adrp_add(out, "x12", "alya_catch_stack_sp", os);
    out.push_str("    str x13, [x12, x10, lsl #3]\n");
    emit_adrp_add(out, "x12", "alya_catch_stack_bp", os);
    out.push_str("    str x29, [x12, x10, lsl #3]\n");
    out.push_str("    add x10, x10, #1\n");
    out.push_str("    str x10, [x9]\n");
}

pub fn emit_try_end(out: &mut String, end_label: &str, stack_delta: i32, os: OperatingSystem) {
    emit_adrp_add(out, "x9", "alya_catch_idx", os);
    out.push_str("    ldr x10, [x9]\n");
    out.push_str("    sub x10, x10, #1\n");
    out.push_str("    str x10, [x9]\n");
    if stack_delta > 0 {
        out.push_str(&format!("    add sp, sp, #{}\n", stack_delta));
    }
    out.push_str(&format!("    b {}\n", end_label));
}

pub fn emit_catch_begin(out: &mut String, catch_label: &str) {
    out.push_str(&format!("{}:\n", catch_label));
}

pub fn emit_catch_load_err(out: &mut String, os: OperatingSystem) {
    emit_adrp_add(out, "x9", "alya_err_msg", os);
    out.push_str("    ldr x0, [x9]\n");
}

pub fn emit_catch_end(out: &mut String, stack_delta: i32) {
    if stack_delta > 0 {
        out.push_str(&format!("    add sp, sp, #{}\n", stack_delta));
    }
}

pub fn emit_array_new(out: &mut String, count: usize) {
    out.push_str(&format!("    mov x0, #{}\n", count));
    out.push_str("    bl alya_array_new\n");
}

pub fn emit_array_set_imm(out: &mut String, index: usize) {
    out.push_str("    ldr x1, [sp]\n");
    out.push_str("    ldr x1, [x1, #16]\n");
    out.push_str(&format!("    mov x2, #{}\n", index * 8));
    out.push_str("    str x0, [x1, x2]\n");
}

pub fn emit_array_get(out: &mut String) {
    out.push_str("    mov x1, x0\n");
    out.push_str("    ldr x0, [sp], #16\n");
    out.push_str("    ldr x2, [x0]\n");
    out.push_str("    cmp x1, #0\n");
    out.push_str("    b.ge 1f\n");
    out.push_str("    add x1, x1, x2\n");
    out.push_str("1:\n");
    out.push_str("    cmp x1, x2\n");
    out.push_str("    b.hs alya_error_index_out_of_bounds\n");
    out.push_str("    ldr x0, [x0, #16]\n");
    out.push_str("    ldr x0, [x0, x1, lsl #3]\n");
}

pub fn emit_array_set(out: &mut String) {
    out.push_str("    mov x2, x0\n");
    out.push_str("    ldr x1, [sp], #16\n");
    out.push_str("    ldr x0, [sp], #16\n");
    out.push_str("    ldr x3, [x0]\n");
    out.push_str("    cmp x1, #0\n");
    out.push_str("    b.ge 1f\n");
    out.push_str("    add x1, x1, x3\n");
    out.push_str("1:\n");
    out.push_str("    cmp x1, x3\n");
    out.push_str("    b.hs alya_error_index_out_of_bounds\n");
    out.push_str("    ldr x0, [x0, #16]\n");
    out.push_str("    str x2, [x0, x1, lsl #3]\n");
}

pub fn emit_array_push(out: &mut String) {
    out.push_str("    mov x1, x0\n");
    out.push_str("    ldr x0, [sp], #16\n");
    out.push_str("    bl alya_array_push\n");
}

pub fn emit_array_pop(out: &mut String) {
    out.push_str("    bl alya_array_pop\n");
}

pub fn emit_array_len(out: &mut String) {
    out.push_str("    cbz x0, 1f\n");
    out.push_str("    ldr x0, [x0]\n");
    out.push_str("1:\n");
}

pub fn emit_print_array(out: &mut String) {
    out.push_str("    bl alya_print_array\n");
}

pub fn emit_print_map(out: &mut String) {
    out.push_str("    bl alya_print_map\n");
}

pub fn emit_struct_new(
    out: &mut String,
    desc_label: &str,
    field_count: usize,
    os: OperatingSystem,
) {
    emit_adrp_add(out, "x0", desc_label, os);
    out.push_str(&format!("    mov x1, #{}\n", field_count));
    out.push_str("    bl alya_struct_new\n");
}

pub fn emit_fat_ptr_new(out: &mut String, vtable_label: &str, os: OperatingSystem) {
    emit_adrp_add(out, "x1", vtable_label, os);
    let p = if matches!(os, OperatingSystem::MacOS) {
        "_"
    } else {
        ""
    };
    out.push_str(&format!("    bl {}alya_fat_ptr_new\n", p));
}

pub fn emit_struct_field_get(out: &mut String, field_idx: usize) {
    out.push_str(&format!("    ldr x0, [x0, #{}]\n", (field_idx + 1) * 8));
}

pub fn emit_struct_field_set_imm(out: &mut String, field_idx: usize) {
    out.push_str("    ldr x1, [sp]\n");
    out.push_str(&format!("    str x0, [x1, #{}]\n", (field_idx + 1) * 8));
}

pub fn emit_struct_field_set(out: &mut String, field_idx: usize) {
    out.push_str("    ldr x1, [sp], #16\n");
    out.push_str(&format!("    str x0, [x1, #{}]\n", (field_idx + 1) * 8));
}

pub fn emit_print_struct(out: &mut String) {
    out.push_str("    bl alya_print_struct\n");
}

#[allow(clippy::too_many_arguments)]
pub fn emit_for_each_load_element(
    out: &mut String,
    arr_offset: i32,
    idx_offset: i32,
    var_offset: i32,
    val_offset: Option<i32>,
    end_label: &str,
    map_label: &str,
    done_label: &str,
) {
    emit_arm64_load_x29_offset(out, "x0", arr_offset, "x9");
    out.push_str(&format!("    cbz x0, {}\n", end_label));
    out.push_str("    ldr x2, [x0, #-16]\n");
    out.push_str("    mov x3, #0x0002\n");
    out.push_str("    movk x3, #0x5A11, lsl #16\n");
    out.push_str("    cmp x2, x3\n");
    out.push_str(&format!("    b.eq {}\n", map_label));

    // ARRAY
    out.push_str("    ldr x1, [x0]\n");
    emit_arm64_load_x29_offset(out, "x2", idx_offset, "x9");
    out.push_str("    cmp x2, x1\n");
    out.push_str(&format!("    b.ge {}\n", end_label));
    out.push_str("    ldr x3, [x0, #16]\n");
    out.push_str("    ldr x4, [x3, x2, lsl #3]\n");
    if let Some(v_off) = val_offset {
        emit_arm64_store_x29_offset(out, "x2", var_offset, "x9");
        emit_arm64_store_x29_offset(out, "x4", v_off, "x9");
    } else {
        emit_arm64_store_x29_offset(out, "x4", var_offset, "x9");
    }
    out.push_str(&format!("    b {}\n", done_label));

    // MAP
    out.push_str(&format!("{}:\n", map_label));
    let scan_label = format!("{}_scan", map_label);
    let found_label = format!("{}_found", map_label);
    out.push_str("    ldr x1, [x0, #8]\n");
    emit_arm64_load_x29_offset(out, "x2", idx_offset, "x9");
    out.push_str(&format!("{}:\n", scan_label));
    out.push_str("    cmp x2, x1\n");
    out.push_str(&format!("    b.ge {}\n", end_label));
    out.push_str("    ldr x3, [x0, #16]\n");
    out.push_str("    add x4, x2, x2, lsl #1\n");
    out.push_str("    add x4, x3, x4, lsl #3\n");
    out.push_str("    ldr x5, [x4, #16]\n");
    out.push_str("    cmp w5, #1\n");
    out.push_str(&format!("    b.eq {}\n", found_label));
    out.push_str("    add x2, x2, #1\n");
    out.push_str(&format!("    b {}\n", scan_label));
    out.push_str(&format!("{}:\n", found_label));
    emit_arm64_store_x29_offset(out, "x2", idx_offset, "x9");
    out.push_str("    ldr x6, [x4]\n");
    out.push_str("    ldr x7, [x4, #8]\n");
    if let Some(v_off) = val_offset {
        emit_arm64_store_x29_offset(out, "x6", var_offset, "x9");
        emit_arm64_store_x29_offset(out, "x7", v_off, "x9");
    } else {
        emit_arm64_store_x29_offset(out, "x6", var_offset, "x9");
    }

    out.push_str(&format!("{}:\n", done_label));
}

pub fn emit_string_equality_call(out: &mut String, op: BinaryOp) {
    out.push_str("    mov x1, x0\n");
    out.push_str("    ldr x0, [sp], #16\n");
    out.push_str("    bl fn_strcmp\n");
    match op {
        BinaryOp::Equal => {
            out.push_str("    cmp x0, #0\n    cset x0, eq\n");
        }
        BinaryOp::NotEqual => {
            out.push_str("    cmp x0, #0\n    cset x0, ne\n");
        }
        BinaryOp::Less => {
            out.push_str("    cmp x0, #0\n    cset x0, lt\n");
        }
        BinaryOp::LessEqual => {
            out.push_str("    cmp x0, #0\n    cset x0, le\n");
        }
        BinaryOp::Greater => {
            out.push_str("    cmp x0, #0\n    cset x0, gt\n");
        }
        BinaryOp::GreaterEqual => {
            out.push_str("    cmp x0, #0\n    cset x0, ge\n");
        }
        _ => {}
    }
}

pub fn emit_in_call(out: &mut String, op: BinaryOp) {
    // x0 is collection, item was pushed to stack
    out.push_str("    ldr x1, [sp], #16\n"); // x1 = item, x0 = collection
    out.push_str("    bl fn_in\n");
    if op == BinaryOp::NotIn {
        out.push_str("    cmp x0, #0\n    cset x0, eq\n");
    }
}

pub fn emit_char_code_at(out: &mut String, done_label: &str) {
    out.push_str("    mov x1, x0\n");
    out.push_str("    ldr x2, [sp], #16\n");
    out.push_str("    mov x0, #0\n");
    out.push_str(&format!("    cbz x2, {}\n", done_label));
    out.push_str(&format!("    tbnz x1, #63, {}\n", done_label));
    out.push_str("    ldrb w0, [x2, x1]\n");
    out.push_str(&format!("{}:\n", done_label));
}

pub fn emit_char_code_at_direct(out: &mut String, done_label: &str) {
    out.push_str("    mov x0, #0\n");
    out.push_str(&format!("    cbz x2, {}\n", done_label));
    out.push_str(&format!("    tbnz x1, #63, {}\n", done_label));
    out.push_str("    ldrb w0, [x2, x1]\n");
    out.push_str(&format!("{}:\n", done_label));
}

pub fn emit_call_str_to_int(out: &mut String) {
    out.push_str("    bl fn_str_to_int\n");
}

pub fn emit_call_str_to_float(out: &mut String) {
    out.push_str("    bl fn_str_to_float\n");
}

pub fn emit_rc_retain(out: &mut String) {
    out.push_str("    bl fn_rc_retain\n");
}

pub fn emit_rc_release(out: &mut String) {
    out.push_str("    bl fn_rc_release\n");
}

pub fn emit_rc_release_stack(out: &mut String, offset: i32) {
    emit_arm64_load_x29_offset(out, "x0", offset, "x9");
    out.push_str("    bl fn_rc_release\n");
}

pub fn emit_weak_check(out: &mut String, lbl: &str) {
    let clean_lbl = lbl.trim_start_matches('.');
    out.push_str(&format!("    cbz x0, .L_weak_done_{}\n", clean_lbl));
    out.push_str("    str x0, [sp, #-16]!\n");
    out.push_str("    bl fn_rc_count\n");
    out.push_str("    ldr x1, [sp], #16\n");
    out.push_str(&format!("    cbnz x0, .L_weak_alive_{}\n", clean_lbl));
    out.push_str("    mov x0, #0\n");
    out.push_str(&format!("    b .L_weak_done_{}\n", clean_lbl));
    out.push_str(&format!(".L_weak_alive_{}:\n", clean_lbl));
    out.push_str("    mov x0, x1\n");
    out.push_str(&format!(".L_weak_done_{}:\n", clean_lbl));
}
