pub mod alloc;
pub mod arena;
pub mod arrays;
pub mod errors;
pub mod fs;
pub mod gc;
pub mod heap;
pub mod io;
pub mod maps;
pub mod math;
pub mod net;
pub mod str_ops;
pub mod str_split;
pub mod structs;
pub mod thread;
pub mod fiber;

use crate::codegen::target::OperatingSystem;

pub fn emit_x86_runtime(out: &mut String, os: OperatingSystem) {
    alloc::emit(out, os);
    str_ops::emit(out, os);
    str_split::emit(out, os);
    io::emit(out, os);
    math::emit(out, os);
    fs::emit(out, os);
    arrays::emit(out, os);
    maps::emit(out, os);
    structs::emit(out, os);
    heap::emit(out, os);
    gc::emit(out, os);
    arena::emit(out, os);
    net::emit(out, os);
    thread::emit(out, os);
    fiber::emit(out, os);
    errors::emit(out, os);
}

pub(crate) fn emit_str_buf_load(out: &mut String, buf_reg: &str, idx_reg: &str, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("    movl %fs:0x14, %eax\n");
        out.push_str("    andl $63, %eax\n");
    } else {
        out.push_str("    movl %gs:0, %eax\n");
        out.push_str("    shrl $12, %eax\n");
        out.push_str("    andl $63, %eax\n");
    }
    out.push_str(&format!("    movl alya_str_idx(,%eax,4), {}\n", idx_reg));
    out.push_str("    shll $20, %eax\n");
    out.push_str(&format!("    movl $alya_str_buf, {}\n", buf_reg));
    out.push_str(&format!("    addl %eax, {}\n", buf_reg));
}

pub(crate) fn emit_str_buf_store(out: &mut String, idx_val_reg: &str, scratch_reg: &str, os: OperatingSystem) {
    if matches!(os, OperatingSystem::Windows) {
        out.push_str(&format!("    movl %fs:0x14, {}\n", scratch_reg));
        out.push_str(&format!("    andl $63, {}\n", scratch_reg));
    } else {
        out.push_str(&format!("    movl %gs:0, {}\n", scratch_reg));
        out.push_str(&format!("    shrl $12, {}\n", scratch_reg));
        out.push_str(&format!("    andl $63, {}\n", scratch_reg));
    }
    out.push_str(&format!("    movl {}, alya_str_idx(,{scratch_reg},4)\n", idx_val_reg));
}
