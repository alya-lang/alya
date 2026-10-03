use crate::codegen::target::OperatingSystem;

// B3: crash diagnostics. alya_crash_init installs a fault handler first
// thing in main; the handler prints a bare trap message to stderr and
// exits 134. Only async-signal-safe calls inside (write/_exit on Unix,
// WriteFile/ExitProcess on Windows): printf/malloc/atexit-report would
// deadlock or re-enter on a corrupted heap. No source location is
// available without debug tables; the message names the fault instead.
#[rustfmt::skip]
pub fn emit(out: &mut String, os: OperatingSystem) {
    out.push_str(".global alya_crash_init\n");
    out.push_str("alya_crash_init:\n");
    out.push_str("    push %rbp\n");
    out.push_str("    mov %rsp, %rbp\n");
    if matches!(os, OperatingSystem::Windows) {
        // Vectored handlers catch hardware AVs; MSVCRT signal() does not.
        out.push_str("    lea alya_crash_handler_win(%rip), %rdx\n");
        out.push_str("    mov $1, %rcx\n");
        out.push_str("    sub $32, %rsp\n");
        out.push_str("    call AddVectoredExceptionHandler\n");
        out.push_str("    add $32, %rsp\n");
    } else {
        // Unix: signal(signo, handler) for the fault signals.
        // Linux: SEGV/ILL/FPE/BUS/ABRT = 11/4/8/7/6; macOS BUS = 10.
        let bus = if matches!(os, OperatingSystem::MacOS) { 10 } else { 7 };
        let p = if matches!(os, OperatingSystem::MacOS) { "_" } else { "" };
        for sig in [11, 4, 8, bus, 6] {
            out.push_str(&format!("    mov ${}, %rdi\n", sig));
            out.push_str("    lea alya_crash_handler(%rip), %rsi\n");
            out.push_str(&format!("    call {}signal\n", p));
        }
    }
    out.push_str("    mov %rbp, %rsp\n");
    out.push_str("    pop %rbp\n");
    out.push_str("    ret\n\n");

    if matches!(os, OperatingSystem::Windows) {
        emit_win_handler(out);
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
    out.push_str("alya_crash_handler:\n");
    // edi = signo. Force-align the stack: handler entry alignment is not
    // guaranteed, and the write/_exit calls below assume an aligned base.
    // Never returns, so clobbering the interrupted frame is safe.
    out.push_str("    and $-16, %rsp\n");
    out.push_str("    cmp $11, %edi\n");
    out.push_str("    je .L_x64_crash_segv\n");
    out.push_str("    cmp $4, %edi\n");
    out.push_str("    je .L_x64_crash_ill\n");
    out.push_str("    cmp $8, %edi\n");
    out.push_str("    je .L_x64_crash_fpe\n");
    out.push_str(&format!("    cmp ${}, %edi\n", bus));
    out.push_str("    je .L_x64_crash_bus\n");
    out.push_str("    cmp $6, %edi\n");
    out.push_str("    je .L_x64_crash_abrt\n");
    out.push_str("    lea alya_crash_other(%rip), %rsi\n");
    out.push_str(&format!("    mov ${}, %rdx\n", super::super::data::CRASH_OTHER.len()));
    out.push_str("    jmp .L_x64_crash_write\n");
    out.push_str(".L_x64_crash_segv:\n");
    out.push_str("    lea alya_crash_segv(%rip), %rsi\n");
    out.push_str(&format!("    mov ${}, %rdx\n", super::super::data::CRASH_SEGV.len()));
    out.push_str("    jmp .L_x64_crash_write\n");
    out.push_str(".L_x64_crash_ill:\n");
    out.push_str("    lea alya_crash_ill(%rip), %rsi\n");
    out.push_str(&format!("    mov ${}, %rdx\n", super::super::data::CRASH_ILL.len()));
    out.push_str("    jmp .L_x64_crash_write\n");
    out.push_str(".L_x64_crash_fpe:\n");
    out.push_str("    lea alya_crash_fpe(%rip), %rsi\n");
    out.push_str(&format!("    mov ${}, %rdx\n", super::super::data::CRASH_FPE.len()));
    out.push_str("    jmp .L_x64_crash_write\n");
    out.push_str(".L_x64_crash_bus:\n");
    out.push_str("    lea alya_crash_bus(%rip), %rsi\n");
    out.push_str(&format!("    mov ${}, %rdx\n", super::super::data::CRASH_BUS.len()));
    out.push_str("    jmp .L_x64_crash_write\n");
    out.push_str(".L_x64_crash_abrt:\n");
    out.push_str("    lea alya_crash_abrt(%rip), %rsi\n");
    out.push_str(&format!("    mov ${}, %rdx\n", super::super::data::CRASH_ABRT.len()));
    out.push_str(".L_x64_crash_write:\n");
    out.push_str("    mov $2, %edi\n");
    out.push_str(&format!("    call {}write\n", p));
    out.push_str("    mov $134, %edi\n");
    out.push_str(&format!("    call {}\n", ex));
    out.push_str("    hlt\n\n");
}

#[rustfmt::skip]
fn emit_win_handler(out: &mut String) {
    out.push_str(".global alya_crash_handler_win\n");
    out.push_str("alya_crash_handler_win:\n");
    // rcx = EXCEPTION_POINTERS*. ExceptionRecord* is its first word and
    // the ExceptionCode the record's first word.
    out.push_str("    and $-16, %rsp\n");
    out.push_str("    mov (%rcx), %r10\n");
    out.push_str("    mov (%r10), %eax\n");
    out.push_str("    cmp $0xC0000005, %eax\n");
    out.push_str("    je .L_x64_crash_win_av\n");
    out.push_str("    cmp $0xC0000094, %eax\n");
    out.push_str("    je .L_x64_crash_win_div\n");
    out.push_str("    cmp $0xC000001D, %eax\n");
    out.push_str("    je .L_x64_crash_win_ill\n");
    // Unknown codes (e.g. stack overflow, where this stack is unusable
    // anyway): pass along instead of swallowing the crash.
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    ret\n");
    out.push_str(".L_x64_crash_win_av:\n");
    out.push_str("    lea alya_crash_win_av(%rip), %rdx\n");
    out.push_str(&format!("    mov ${}, %r8\n", super::super::data::CRASH_WIN_AV.len()));
    out.push_str("    jmp .L_x64_crash_win_write\n");
    out.push_str(".L_x64_crash_win_div:\n");
    out.push_str("    lea alya_crash_win_div(%rip), %rdx\n");
    out.push_str(&format!("    mov ${}, %r8\n", super::super::data::CRASH_WIN_DIV.len()));
    out.push_str("    jmp .L_x64_crash_win_write\n");
    out.push_str(".L_x64_crash_win_ill:\n");
    out.push_str("    lea alya_crash_win_ill(%rip), %rdx\n");
    out.push_str(&format!("    mov ${}, %r8\n", super::super::data::CRASH_WIN_ILL.len()));
    out.push_str(".L_x64_crash_win_write:\n");
    // WriteFile(GetStdHandle(-12), msg, len, &written, NULL);
    // ExitProcess(134). One sub covers both calls' shadow space; msg/len
    // ride on the stack because GetStdHandle clobbers every volatile reg.
    out.push_str("    sub $64, %rsp\n");
    out.push_str("    mov %rdx, 40(%rsp)\n");
    out.push_str("    mov %r8, 48(%rsp)\n");
    out.push_str("    mov $-12, %rcx\n");
    out.push_str("    call GetStdHandle\n");
    out.push_str("    mov %rax, %rcx\n");
    out.push_str("    mov 40(%rsp), %rdx\n");
    out.push_str("    mov 48(%rsp), %r8\n");
    out.push_str("    lea 56(%rsp), %r9\n");
    out.push_str("    movq $0, 32(%rsp)\n");
    out.push_str("    call WriteFile\n");
    out.push_str("    mov $134, %rcx\n");
    out.push_str("    call ExitProcess\n");
    out.push_str("    hlt\n\n");
}
