use crate::codegen::target::OperatingSystem;

#[rustfmt::skip]
pub fn emit(out: &mut String, _os: OperatingSystem) {
    // =========================================================================
    // fn_gc_add_purple(ptr) - x86 (32-bit)
    // =========================================================================
    out.push_str(".global fn_gc_add_purple\n");
    out.push_str("fn_gc_add_purple:\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_gc_collect() / fn__gc_collect() -> int - x86 (32-bit)
    // =========================================================================
    out.push_str(".global fn_gc_collect\n");
    out.push_str("fn_gc_collect:\n");
    out.push_str(".global fn__gc_collect\n");
    out.push_str("fn__gc_collect:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    ret\n\n");

    // =========================================================================
    // fn_gc_collected_count() / fn__gc_collected_count() -> int - x86 (32-bit)
    // =========================================================================
    out.push_str(".global fn_gc_collected_count\n");
    out.push_str("fn_gc_collected_count:\n");
    out.push_str(".global fn__gc_collected_count\n");
    out.push_str("fn__gc_collected_count:\n");
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    ret\n\n");
}
