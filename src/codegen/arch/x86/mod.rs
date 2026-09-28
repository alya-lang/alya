pub mod builtins;
pub mod control;
pub mod loads;
pub mod ops;
pub mod say;

pub use builtins::*;
pub use control::*;
pub use loads::*;
pub use ops::*;
pub use say::*;

use crate::codegen::target::OperatingSystem;

pub fn emit_header(out: &mut String, os: OperatingSystem) {
    // 32-bit Windows decorates C symbols with a leading underscore:
    // the entry point must be `_main` there (plain `main` on ELF).
    if matches!(os, OperatingSystem::Windows) {
        out.push_str(".global _main\n");
    } else {
        out.push_str(".global main\n");
    }
    out.push_str(".extern printf\n");
    out.push_str(".extern exit\n");
    out.push_str(".extern atexit\n");
    out.push_str(".extern getchar\n");
    out.push_str(".extern fflush\n");
    out.push_str(".extern calloc\n");
    out.push_str(".extern malloc\n");
    out.push_str(".extern free\n");
    out.push_str(".extern realloc\n");
    out.push_str(".extern memcpy\n");
    out.push_str(".extern memset\n");
    out.push_str(".extern time\n");
    out.push_str(".extern getenv\n");
    out.push_str(".extern system\n");
    out.push_str(".extern Sleep\n");
    out.push_str(".extern GetTickCount\n");
    out.push_str(".extern usleep\n");
    out.push_str(".extern clock_gettime\n");
    out.push_str(".extern _mkdir\n");
    out.push_str(".extern mkdir\n");
    out.push_str(".extern socket\n");
    out.push_str(".extern connect\n");
    out.push_str(".extern send\n");
    out.push_str(".extern recv\n");
    out.push_str(".extern close\n");
    out.push_str(".extern closesocket\n");
    out.push_str(".extern bind\n");
    out.push_str(".extern listen\n");
    out.push_str(".extern accept\n");
    out.push_str(".extern htons\n");
    out.push_str(".extern inet_addr\n");
    out.push_str(".extern gethostbyname\n");
    out.push_str(".extern setsockopt\n");
    out.push_str(".extern getpeername\n");
    out.push_str(".extern inet_ntoa\n");
    out.push_str(".extern sendto\n");
    out.push_str(".extern recvfrom\n");
    out.push_str(".extern select\n");
    out.push_str(".extern ioctlsocket\n");
    out.push_str(".extern fcntl\n");
    out.push_str(".extern sin\n");
    out.push_str(".extern cos\n");
    out.push_str(".extern tan\n");
    out.push_str(".extern asin\n");
    out.push_str(".extern acos\n");
    out.push_str(".extern atan\n");
    out.push_str(".extern atan2\n");
    out.push_str(".extern sinh\n");
    out.push_str(".extern cosh\n");
    out.push_str(".extern tanh\n");
    out.push_str(".extern log\n");
    out.push_str(".extern log2\n");
    out.push_str(".extern log10\n");
    out.push_str(".extern exp\n");
    out.push_str(".extern fmod\n");
    out.push_str(".extern sqrt\n");
    out.push_str(".extern ceil\n");
    out.push_str(".extern floor\n");
    out.push_str(".extern FindFirstFileA\n");
    out.push_str(".extern FindNextFileA\n");
    out.push_str(".extern FindClose\n");
    out.push_str(".extern opendir\n");
    out.push_str(".extern closedir\n");
    out.push_str(".extern CreateThread\n");
    out.push_str(".extern WaitForSingleObject\n");
    out.push_str(".extern CloseHandle\n");
    out.push_str(".extern GetCurrentThreadId\n");
    out.push_str(".extern CreateMutexA\n");
    out.push_str(".extern ReleaseMutex\n");
    out.push_str(".extern pthread_create\n");
    out.push_str(".extern pthread_join\n");
    out.push_str(".extern pthread_self\n");
    out.push_str(".extern pthread_mutex_init\n");
    out.push_str(".extern pthread_mutex_lock\n");
    out.push_str(".extern pthread_mutex_unlock\n");
    out.push_str(".extern pthread_mutex_destroy\n");
    out.push_str(".extern GetCurrentProcessId\n");
    out.push_str(".extern GetCurrentDirectoryA\n");
    out.push_str(".extern SetCurrentDirectoryA\n");
    out.push_str(".extern GetFileAttributesA\n");
    out.push_str(".extern _rmdir\n");
    out.push_str(".extern getpid\n");
    out.push_str(".extern getcwd\n");
    out.push_str(".extern chdir\n");
    out.push_str(".extern rmdir\n\n");
    out.push_str(".text\n");
    // Entry label matches the .global above (underscore on Windows).
    if matches!(os, OperatingSystem::Windows) {
        out.push_str("_main:\n");
    } else {
        out.push_str("main:\n");
    }
    out.push_str("    push %ebp\n");
    out.push_str("    mov %esp, %ebp\n");
    out.push_str("    movl 8(%ebp), %eax\n");
    out.push_str("    movl %eax, alya_argc\n");
    out.push_str("    movl 12(%ebp), %eax\n");
    out.push_str("    movl %eax, alya_argv\n\n");
}

pub fn emit_footer(out: &mut String) {
    out.push_str("    xor %eax, %eax\n");
    out.push_str("    mov %ebp, %esp\n");
    out.push_str("    pop %ebp\n");
    out.push_str("    ret\n");
}

/// Win32 stdcall arities in bytes (args * 4), for `_name@N` decoration.
fn stdcall_arity(name: &str) -> Option<u32> {
    Some(match name {
        "GetTickCount" | "GetCurrentThreadId" | "GetCurrentProcessId" => 0,
        "Sleep"
        | "FindClose"
        | "GetFileAttributesA"
        | "SetCurrentDirectoryA"
        | "CloseHandle"
        | "ReleaseMutex"
        | "closesocket"
        | "htons"
        | "inet_addr"
        | "gethostbyname"
        | "inet_ntoa" => 4,
        "FindFirstFileA"
        | "FindNextFileA"
        | "WaitForSingleObject"
        | "GetCurrentDirectoryA"
        | "listen" => 8,
        "socket" | "connect" | "bind" | "accept" | "getpeername" | "CreateMutexA"
        | "ioctlsocket" => 12,
        "send" | "recv" => 16,
        "setsockopt" | "select" => 20,
        "CreateThread" | "sendto" | "recvfrom" => 24,
        _ => return None,
    })
}

/// Decorates external symbols for 32-bit Windows: cdecl imports take a
/// leading underscore (`printf` -> `_printf`), stdcall imports take
/// `_name@N`. Applied as a final text pass over the generated assembly
/// (see `generate_full`); ELF needs no decoration. Symbols defined
/// in-unit (runtime fns, BSS, user functions, entry) are left alone, so
/// FFI `call my_c_fn` references resolve against gcc-built objects too.
/// cdecl is assumed for unknown externals; stdcall user APIs keep working
/// only when listed above (stdlib coverage) or spelled `_name@N` by hand.
pub fn decorate_windows_externals(asm: &str) -> String {
    use std::collections::HashSet;
    let mut defined: HashSet<&str> = HashSet::new();
    for line in asm.lines() {
        let t = line.trim_start();
        if let Some(rest) = t
            .strip_prefix(".global")
            .or_else(|| t.strip_prefix(".globl"))
        {
            let name = rest.split_whitespace().next().unwrap_or("");
            if !name.is_empty() {
                defined.insert(name);
            }
        } else if let Some(colon) = t.find(':') {
            let name = t[..colon].trim();
            if !name.is_empty() && !name.starts_with('.') && !name.contains([' ', '\t']) {
                defined.insert(name);
            }
        }
    }
    let mut out = String::with_capacity(asm.len() + 512);
    for line in asm.lines() {
        let t = line.trim_start();
        let sym: Option<&str> = if let Some(rest) = t.strip_prefix("call ") {
            rest.split_whitespace().next()
        } else if let Some(rest) = t.strip_prefix("jmp ") {
            rest.split_whitespace().next()
        } else if let Some(rest) = t.strip_prefix(".extern ") {
            rest.split_whitespace().next()
        } else {
            None
        };
        let mut rewritten = line.to_string();
        if let Some(sym) = sym {
            // An existing `@N` suffix is authoritative for the arity;
            // only the missing `_` prefix is added. Fully decorated
            // `_name@N` references are never touched.
            let (bare, suffix) = match sym.split_once('@') {
                Some((b, s)) => (b, Some(s)),
                None => (sym, None),
            };
            let needs_deco = !(sym.starts_with(['.', '*', '$', '%'])
                || bare.is_empty()
                || bare.starts_with(|c: char| c.is_ascii_digit())
                || defined.contains(bare)
                || defined.contains(sym)
                || bare.starts_with('_') && suffix.is_some());
            if needs_deco {
                let decorated = if let Some(name) = bare.strip_prefix('_') {
                    match stdcall_arity(name) {
                        Some(n) => format!("{}@{}", bare, n),
                        None => sym.to_string(),
                    }
                } else {
                    match suffix {
                        Some(s) => format!("_{}@{}", bare, s),
                        None => match stdcall_arity(bare) {
                            Some(n) => format!("_{}@{}", bare, n),
                            None => format!("_{}", bare),
                        },
                    }
                };
                // Whole-word replacement within the line only.
                let mut fresh = String::with_capacity(line.len() + 8);
                let mut rest = line;
                while let Some(pos) = rest.find(sym) {
                    let before_ok = pos == 0
                        || !rest.as_bytes()[pos - 1].is_ascii_alphanumeric()
                            && rest.as_bytes()[pos - 1] != b'_';
                    let after = pos + sym.len();
                    let after_ok = after >= rest.len()
                        || !rest.as_bytes()[after].is_ascii_alphanumeric()
                            && rest.as_bytes()[after] != b'_'
                            && rest.as_bytes()[after] != b'@';
                    fresh.push_str(&rest[..pos]);
                    if before_ok && after_ok {
                        fresh.push_str(&decorated);
                    } else {
                        fresh.push_str(sym);
                    }
                    rest = &rest[after..];
                }
                fresh.push_str(rest);
                rewritten = fresh;
            }
        }
        out.push_str(&rewritten);
        out.push('\n');
    }
    out
}
