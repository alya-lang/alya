use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, _os: OperatingSystem) {
    // =========================================================================
    // fn_gc_add_purple(ptr) - ARM64
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn_gc_add_purple\n");
    out.push_str("fn_gc_add_purple:\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_gc_collect() / fn__gc_collect() -> int - ARM64
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn_gc_collect\n");
    out.push_str("fn_gc_collect:\n");
    out.push_str(".global fn__gc_collect\n");
    out.push_str("fn__gc_collect:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_gc_collected_count() / fn__gc_collected_count() -> int - ARM64
    // =========================================================================
    out.push_str(".align 2\n");
    out.push_str(".global fn_gc_collected_count\n");
    out.push_str("fn_gc_collected_count:\n");
    out.push_str(".global fn__gc_collected_count\n");
    out.push_str("fn__gc_collected_count:\n");
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n\n");
}
