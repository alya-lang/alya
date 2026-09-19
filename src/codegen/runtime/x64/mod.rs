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

use crate::codegen::target::OperatingSystem;

pub fn emit_x64_runtime(out: &mut String, os: OperatingSystem) {
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
    errors::emit(out, os);
}

pub fn emit_str_buf_ctx(out: &mut String, os: OperatingSystem) {
    match os {
        OperatingSystem::Windows => {
            out.push_str("    mov %gs:0x28, %r11\n");
            out.push_str("    and $63, %r11\n");
            out.push_str("    lea alya_str_idx(%rip), %r9\n");
            out.push_str("    lea (%r9, %r11, 8), %r9\n");
            out.push_str("    shl $20, %r11\n");
            out.push_str("    lea alya_str_buf(%rip), %r8\n");
            out.push_str("    add %r11, %r8\n");
        }
        OperatingSystem::Linux => {
            out.push_str("    mov %fs:0, %r11\n");
            out.push_str("    shr $12, %r11\n");
            out.push_str("    and $63, %r11d\n");
            out.push_str("    lea alya_str_idx(%rip), %r9\n");
            out.push_str("    lea (%r9, %r11, 8), %r9\n");
            out.push_str("    shl $20, %r11\n");
            out.push_str("    lea alya_str_buf(%rip), %r8\n");
            out.push_str("    add %r11, %r8\n");
        }
        OperatingSystem::MacOS => {
            out.push_str("    mov %gs:0, %r11\n");
            out.push_str("    shr $12, %r11\n");
            out.push_str("    and $63, %r11d\n");
            out.push_str("    lea alya_str_idx(%rip), %r9\n");
            out.push_str("    lea (%r9, %r11, 8), %r9\n");
            out.push_str("    shl $20, %r11\n");
            out.push_str("    lea alya_str_buf(%rip), %r8\n");
            out.push_str("    add %r11, %r8\n");
        }
    }
}

