use crate::codegen::target::OperatingSystem;

// B3: crash diagnostics (see x64/crash.rs for the contract). Unix uses
// libc signal/write/_exit; Windows uses AddVectoredExceptionHandler plus
// WriteFile/ExitProcess. Handler code is async-signal-safe only.
#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    out.push_str(".global alya_crash_init\n");
    out.push_str(".align 2\n");
    out.push_str("alya_crash_init:\n");
    out.push_str("    stp x29, x30, [sp, #-16]!\n");
    out.push_str("    mov x29, sp\n");
    if matches!(os, OperatingSystem::Windows) {
        // Vectored handlers catch hardware AVs; MSVCRT signal() does not.
        super::emit_adrp_add(out, "x1", "alya_crash_handler_win", os);
        out.push_str("    mov x0, #1\n");
        out.push_str("    bl AddVectoredExceptionHandler\n");
    } else {
        // Unix: signal(signo, handler) for the fault signals.
        // Linux: SEGV/ILL/FPE/BUS/ABRT = 11/4/8/7/6; macOS BUS = 10.
        let bus = if matches!(os, OperatingSystem::MacOS) { 10 } else { 7 };
        let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
        for sig in [11, 4, 8, bus, 6] {
            out.push_str(&format!("    mov x0, #{}\n", sig));
            super::emit_adrp_add(out, "x1", "alya_crash_handler", os);
            out.push_str(&format!("    bl {}signal\n", p));
        }
    }
    out.push_str("    ldp x29, x30, [sp], #16\n");
    out.push_str("    ret\n\n");

    if matches!(os, OperatingSystem::Windows) {
        emit_win_handler(out, os);
    } else {
        emit_unix_handler(out, os);
    }
}

#[rustfmt::skip]
fn emit_unix_handler(out: &mut String, os: OperatingSystem) {
    let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
    let bus = if matches!(os, OperatingSystem::MacOS) { 10 } else { 7 };
    let ex = if matches!(os, OperatingSystem::MacOS) { "__exit" } else { "_exit" };
    out.push_str(".global alya_crash_handler\n");
    out.push_str(".align 2\n");
    out.push_str("alya_crash_handler:\n");
    // w0 = signo. Force-align the stack; never returns.
    out.push_str("    mov x9, sp\n");
    out.push_str("    and x9, x9, #-16\n");
    out.push_str("    mov sp, x9\n");
    out.push_str("    cmp w0, #11\n");
    out.push_str("    b.eq .L_arm64_crash_segv\n");
    out.push_str("    cmp w0, #4\n");
    out.push_str("    b.eq .L_arm64_crash_ill\n");
    out.push_str("    cmp w0, #8\n");
    out.push_str("    b.eq .L_arm64_crash_fpe\n");
    out.push_str(&format!("    cmp w0, #{}\n", bus));
    out.push_str("    b.eq .L_arm64_crash_bus\n");
    out.push_str("    cmp w0, #6\n");
    out.push_str("    b.eq .L_arm64_crash_abrt\n");
    super::emit_adrp_add(out, "x1", "alya_crash_other", os);
    out.push_str(&format!("    mov x2, #{}\n", super::super::data::CRASH_OTHER.len()));
    out.push_str("    b .L_arm64_crash_write\n");
    out.push_str(".L_arm64_crash_segv:\n");
    super::emit_adrp_add(out, "x1", "alya_crash_segv", os);
    out.push_str(&format!("    mov x2, #{}\n", super::super::data::CRASH_SEGV.len()));
    out.push_str("    b .L_arm64_crash_write\n");
    out.push_str(".L_arm64_crash_ill:\n");
    super::emit_adrp_add(out, "x1", "alya_crash_ill", os);
    out.push_str(&format!("    mov x2, #{}\n", super::super::data::CRASH_ILL.len()));
    out.push_str("    b .L_arm64_crash_write\n");
    out.push_str(".L_arm64_crash_fpe:\n");
    super::emit_adrp_add(out, "x1", "alya_crash_fpe", os);
    out.push_str(&format!("    mov x2, #{}\n", super::super::data::CRASH_FPE.len()));
    out.push_str("    b .L_arm64_crash_write\n");
    out.push_str(".L_arm64_crash_bus:\n");
    super::emit_adrp_add(out, "x1", "alya_crash_bus", os);
    out.push_str(&format!("    mov x2, #{}\n", super::super::data::CRASH_BUS.len()));
    out.push_str("    b .L_arm64_crash_write\n");
    out.push_str(".L_arm64_crash_abrt:\n");
    super::emit_adrp_add(out, "x1", "alya_crash_abrt", os);
    out.push_str(&format!("    mov x2, #{}\n", super::super::data::CRASH_ABRT.len()));
    out.push_str(".L_arm64_crash_write:\n");
    out.push_str("    mov x0, #2\n");
    out.push_str(&format!("    bl {}write\n", p));
    out.push_str("    mov x0, #134\n");
    out.push_str(&format!("    bl {}\n", ex));
    // Should never reach here; trap if it does.
    out.push_str("    brk #0\n\n");
}

#[rustfmt::skip]
fn emit_win_handler(out: &mut String, os: OperatingSystem) {
    out.push_str(".global alya_crash_handler_win\n");
    out.push_str(".align 2\n");
    out.push_str("alya_crash_handler_win:\n");
    // x0 = EXCEPTION_POINTERS*. ExceptionRecord* is its first word and
    // the ExceptionCode the record's first word.
    // NOTE (alya-lang/alya#109): sp is left UNTOUCHED until the fatal
    // path below. Aligning up front would shift the stack for
    // passthrough codes too, and the OS does not restore sp when a
    // vectored handler returns CONTINUE_SEARCH — subsequent dispatch
    // would read every frame slot shifted and jump wild.
    out.push_str("    ldr x1, [x0]\n");
    out.push_str("    ldr w1, [x1]\n");
    out.push_str("    movz x2, #0x0005\n");
    out.push_str("    movk x2, #0xC000, lsl #16\n");
    out.push_str("    cmp w1, w2\n");
    out.push_str("    b.eq .L_arm64_crash_win_av\n");
    out.push_str("    movz x2, #0x0094\n");
    out.push_str("    movk x2, #0xC000, lsl #16\n");
    out.push_str("    cmp w1, w2\n");
    out.push_str("    b.eq .L_arm64_crash_win_div\n");
    out.push_str("    movz x2, #0x001D\n");
    out.push_str("    movk x2, #0xC000, lsl #16\n");
    out.push_str("    cmp w1, w2\n");
    out.push_str("    b.eq .L_arm64_crash_win_ill\n");
    // Unknown codes (e.g. stack overflow, where this stack is unusable
    // anyway): pass along instead of swallowing the crash.
    out.push_str("    mov x0, #0\n");
    out.push_str("    ret\n");
    out.push_str(".L_arm64_crash_win_av:\n");
    super::emit_adrp_add(out, "x1", "alya_crash_win_av", os);
    out.push_str(&format!("    mov x2, #{}\n", super::super::data::CRASH_WIN_AV.len()));
    out.push_str("    b .L_arm64_crash_win_write\n");
    out.push_str(".L_arm64_crash_win_div:\n");
    super::emit_adrp_add(out, "x1", "alya_crash_win_div", os);
    out.push_str(&format!("    mov x2, #{}\n", super::super::data::CRASH_WIN_DIV.len()));
    out.push_str("    b .L_arm64_crash_win_write\n");
    out.push_str(".L_arm64_crash_win_ill:\n");
    super::emit_adrp_add(out, "x1", "alya_crash_win_ill", os);
    out.push_str(&format!("    mov x2, #{}\n", super::super::data::CRASH_WIN_ILL.len()));
    out.push_str(".L_arm64_crash_win_write:\n");
    // WriteFile(GetStdHandle(-12), msg, len, &written, NULL);
    // ExitProcess(134). x1/x2 already hold msg/len. Fatal path only
    // (never returns): force-align here, after the passthrough decision
    // above left sp pristine (#109).
    out.push_str("    mov x9, sp\n");
    out.push_str("    and x9, x9, #-16\n");
    out.push_str("    mov sp, x9\n");
    out.push_str("    sub sp, sp, #64\n");
    out.push_str("    str x1, [sp, #32]\n");
    out.push_str("    str x2, [sp, #40]\n");
    out.push_str("    movn x0, #11\n");
    out.push_str("    bl GetStdHandle\n");
    out.push_str("    ldr x1, [sp, #32]\n");
    out.push_str("    ldr x2, [sp, #40]\n");
    out.push_str("    add x3, sp, #48\n");
    out.push_str("    mov x4, #0\n");
    out.push_str("    bl WriteFile\n");
    out.push_str("    mov x0, #134\n");
    out.push_str("    bl ExitProcess\n");
    out.push_str("    brk #0\n\n");
}
