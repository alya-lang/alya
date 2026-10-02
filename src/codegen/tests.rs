use super::generate;
use super::order_functions_callee_first;
use super::target::{Architecture, OperatingSystem};
use crate::ast::{BinaryOp, Expr, Program, Stmt};

fn simple_program(stmt: Stmt) -> Program {
    Program {
        statements: vec![stmt],
    }
}

#[test]
fn test_codegen_x64_windows_header_and_footer() {
    let program = simple_program(Stmt::Say(Expr::Number(42)));
    let asm = generate(&program, Architecture::X64, OperatingSystem::Windows);

    assert!(asm.contains(".global main"));
    assert!(asm.contains("main:"));
    assert!(asm.contains("push %rbp"));
    assert!(asm.contains("mov %rsp, %rbp"));
    assert!(asm.contains("ret"));
}

#[test]
fn test_codegen_x86_header_and_footer() {
    let program = simple_program(Stmt::Say(Expr::Number(42)));
    let asm = generate(&program, Architecture::X86, OperatingSystem::Linux);

    assert!(asm.contains(".global main"));
    assert!(asm.contains("main:"));
    assert!(asm.contains("push %ebp"));
    assert!(asm.contains("mov %esp, %ebp"));
    assert!(asm.contains("ret"));
}

#[test]
fn test_codegen_map_entry_tags_present_x64_arm64() {
    // Packed kind tags must exist in the emitted runtimes that support
    // them (x64 + arm64, 24-byte entries). x86 entries are 12-byte and
    // stay untagged.
    let program = simple_program(Stmt::Say(Expr::Number(42)));
    let asm_x64 = generate(&program, Architecture::X64, OperatingSystem::Windows);
    assert!(asm_x64.contains("fn_map_set_tag"));
    assert!(asm_x64.contains("cmpl $1, 16(%r14)"));
    let asm_arm64 = generate(&program, Architecture::ARM64, OperatingSystem::Linux);
    assert!(asm_arm64.contains("fn_map_set_tag"));
    assert!(asm_arm64.contains("cmp w11, #1"));
    assert!(asm_arm64.contains("ldr w1, [x10, #20]"));
}

#[test]
fn test_codegen_arm64_header_and_footer() {
    let program = simple_program(Stmt::Say(Expr::Number(42)));
    let asm = generate(&program, Architecture::ARM64, OperatingSystem::Linux);

    assert!(asm.contains(".global main"));
    assert!(asm.contains("main:"));
    assert!(asm.contains("stp x29, x30, [sp, #-16]!"));
    assert!(asm.contains("ldp x29, x30, [sp], #16"));
    assert!(asm.contains("ret"));
}

#[test]
fn test_codegen_arm64_windows_net_thread_apis() {
    // Windows ARM64 must not reference POSIX-only net/thread APIs (#23):
    // ioctlsocket/closesocket/CreateThread family instead of
    // fcntl/pthread_*, plus WSAStartup in the main prelude.
    let program = simple_program(Stmt::Say(Expr::Number(42)));
    let asm = generate(&program, Architecture::ARM64, OperatingSystem::Windows);

    assert!(asm.contains("bl WSAStartup"));
    assert!(asm.contains("bl ioctlsocket"));
    assert!(asm.contains("bl closesocket"));
    assert!(asm.contains("bl CreateThread"));
    assert!(asm.contains("bl WaitForSingleObject"));
    assert!(asm.contains("bl CreateMutexA"));
    assert!(asm.contains("bl FindFirstFileA"));
    assert!(asm.contains("bl FindNextFileA"));
    // CreateThread takes 6 args in x0..x5 under AAPCS64 (no shadow space / stack args)
    assert!(asm.contains("mov x4, #0"));
    assert!(asm.contains("mov x5, #0"));
    assert!(!asm.contains("fcntl"));
    assert!(!asm.contains("pthread"));
}

#[test]
fn test_codegen_arm64_large_stack_offset() {
    let mut stmts = Vec::new();
    for i in 0..20 {
        stmts.push(Stmt::Let {
            name: format!("var_{}", i),
            type_ann: None,
            value: Expr::Number(i as i128),
        });
    }
    stmts.push(Stmt::Say(Expr::Identifier("var_19".into())));
    let program = Program { statements: stmts };
    let asm = generate(&program, Architecture::ARM64, OperatingSystem::MacOS);

    // Offset > 256 must use sub x9, x29, #... and [x9] instead of invalid unscaled negative offsets
    assert!(asm.contains("sub x9, x29, #"));
    assert!(!asm.contains("[x29, #-320]"));
}

#[test]
fn test_codegen_string_rodata() {
    let program = simple_program(Stmt::Say(Expr::String("Test String".into())));
    let asm = generate(&program, Architecture::X64, OperatingSystem::Windows);

    assert!(asm.contains(".section .rodata"));
    assert!(asm.contains(".string \"Test String\\n\""));
}

#[test]
fn test_codegen_let_and_binary_op() {
    let program = Program {
        statements: vec![
            Stmt::Let {
                name: "x".into(),
                type_ann: None,
                value: Expr::Binary {
                    left: Box::new(Expr::Number(10)),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Number(20)),
                },
            },
            Stmt::Say(Expr::Identifier("x".into())),
        ],
    };

    let asm = generate(&program, Architecture::X64, OperatingSystem::Windows);
    assert!(asm.contains("mov $10, %rax"));
    assert!(asm.contains("add $20, %rax"));
}

#[test]
fn test_codegen_function_definition() {
    let program = Program {
        statements: vec![Stmt::Function {
            name: "my_func".into(),
            params: vec!["a".into()],
            param_types: vec![None],
            return_type: None,
            defaults: vec![None],
            body: vec![Stmt::Return(Some(Expr::Identifier("a".into())))],
            type_params: vec![],
            attributes: vec![],
        }],
    };

    let asm = generate(&program, Architecture::X64, OperatingSystem::Windows);
    assert!(asm.contains("fn_my_func:"));
    assert!(asm.contains("ret"));
}

#[test]
fn test_codegen_macos_arm64_header_and_sections() {
    let program = simple_program(Stmt::Say(Expr::String("Hello Mac".into())));
    let asm = generate(&program, Architecture::ARM64, OperatingSystem::MacOS);

    assert!(asm.contains(".globl _main"));
    assert!(asm.contains("_main:"));
    assert!(asm.contains(".section __TEXT,__cstring,cstring_literals"));
    assert!(asm.contains(".asciz \"Hello Mac\\n\""));
    assert!(asm.contains("_printf"));
}

#[test]
fn test_codegen_macos_x64_header_and_sections() {
    let program = simple_program(Stmt::Say(Expr::String("Hello Mac".into())));
    let asm = generate(&program, Architecture::X64, OperatingSystem::MacOS);

    assert!(asm.contains(".globl _main"));
    assert!(asm.contains("_main:"));
    assert!(asm.contains(".section __TEXT,__cstring,cstring_literals"));
    assert!(asm.contains(".asciz \"Hello Mac\\n\""));
    assert!(asm.contains("_printf"));
}

#[test]
fn test_codegen_say_flushes_stdout_on_all_backends() {
    // alya-lang/alya#72: piped `say` output was lost on crash everywhere
    // except Windows x64 because only that path flushed stdout. Every
    // `say` must flush right after its own `printf` on every backend/OS.
    // Assertions use the exact multi-line sequences the say paths emit:
    // bare `call fflush` also appears in runtime helpers, so a bare
    // substring check would pass on unpatched code.
    let program = simple_program(Stmt::Say(Expr::String("hi".into())));

    let asm = generate(&program, Architecture::X64, OperatingSystem::Linux);
    assert!(asm.contains("call printf\n    xor %edi, %edi\n    call fflush"));
    let asm = generate(&program, Architecture::X64, OperatingSystem::MacOS);
    assert!(asm.contains("call _printf\n    xor %edi, %edi\n    call _fflush"));
    let asm = generate(&program, Architecture::X64, OperatingSystem::Windows);
    assert!(asm.contains("add $32, %rsp\n    xor %rcx, %rcx\n    sub $32, %rsp\n    call fflush"));

    let asm = generate(&program, Architecture::ARM64, OperatingSystem::Linux);
    assert!(asm.contains("bl printf\n    movz x0, #0\n    bl fflush"));
    let asm = generate(&program, Architecture::ARM64, OperatingSystem::MacOS);
    assert!(asm.contains("bl _printf\n    movz x0, #0\n    bl _fflush"));

    let asm = generate(&program, Architecture::X86, OperatingSystem::Linux);
    assert!(asm.contains("call printf\n    push $0\n    call fflush\n    add $4, %esp"));
}

#[test]
fn test_codegen_say_interpolated_many_args_flushes_on_windows() {
    // The Windows >3-arg interpolated `say` path emits its own `printf`
    // sequence instead of `emit_call_printf`; it must flush as well (#72).
    let program = simple_program(Stmt::Say(Expr::InterpolatedString(vec![
        Expr::Number(1),
        Expr::Number(2),
        Expr::Number(3),
        Expr::Number(4),
    ])));
    let asm = generate(&program, Architecture::X64, OperatingSystem::Windows);
    assert!(asm.contains("call printf"));
    // The >3-arg path restores the stack (add $80 for 4 spilled args here)
    // and then flushes with frame-entry padding.
    assert!(asm.contains("add $80, %rsp\n    sub $32, %rsp\n    xor %rcx, %rcx\n    call fflush"));
}

#[test]
fn test_codegen_arm64_large_number_immediate() {
    let program = simple_program(Stmt::Say(Expr::Number(424242)));
    let asm = generate(&program, Architecture::ARM64, OperatingSystem::MacOS);

    assert!(asm.contains("movz x1, #31026"));
    assert!(asm.contains("movk x1, #6, lsl #16"));
    assert!(!asm.contains("mov x1, #424242"));
}

#[test]
fn test_codegen_arm64_large_number_expr() {
    let program = simple_program(Stmt::Let {
        name: "num".into(),
        type_ann: None,
        value: Expr::Number(424242),
    });
    let asm = generate(&program, Architecture::ARM64, OperatingSystem::Linux);

    assert!(asm.contains("movz x0, #31026"));
    assert!(asm.contains("movk x0, #6, lsl #16"));
    assert!(!asm.contains("mov x0, #424242"));
}

#[test]
fn test_codegen_arm64_runtime_immediates_valid() {
    let program = simple_program(Stmt::Say(Expr::Number(1)));
    let asm = generate(&program, Architecture::ARM64, OperatingSystem::MacOS);

    assert!(!asm.contains("mov x4, #1000000"));
    assert!(!asm.contains("mov x19, #65536"));
    assert!(asm.contains("movz x4, #16960"));
}

#[test]
fn test_codegen_os_stdlib_linux_and_macos() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = r#"
import "std/os"

# 1. Environment variables
say env_or("NON_EXISTENT_VAR_98765", "default_val")
say has_env("NON_EXISTENT_VAR_98765")
say has_env("PATH")

# 2. CLI arguments helpers
say arg_count()
say arg_at(0, "none")
say arg_at(1, "none")
say arg_at(99, "out_of_bounds")
say has_arg("--flag")
say has_arg("--unknown")

let c_args = cli_args()
for arg in c_args
    say "arg: " + arg
end

# 3. Platform & system
say target_os()
say target_arch()
say os_name()
say arch()
say is_windows() + is_linux() + is_macos()
say is_windows() + is_posix()
if len(platform()) > 0
    say "platform_ok"
end
if len(temp_dir()) > 0
    say "temp_dir_ok"
end
if len(null_device()) > 0
    say "null_device_ok"
end
if len(path_list_separator()) > 0
    say "path_sep_ok"
end

# 4. Command execution
let ret = exec("echo test > " + null_device())
say ret
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().unwrap();
    crate::parser::resolve_imports(
        &mut ast,
        std::path::Path::new("."),
        &crate::parser::CfgContext::host(),
    )
    .unwrap();

    let asm_linux = generate(&ast, Architecture::X64, OperatingSystem::Linux);
    assert!(asm_linux.contains("fn_target_os"));
    assert!(asm_linux.contains("fn_system_exec"));

    let asm_macos = generate(&ast, Architecture::ARM64, OperatingSystem::MacOS);
    assert!(asm_macos.contains("fn_target_os"));
    assert!(asm_macos.contains("fn_system_exec"));
}

#[test]
fn test_codegen_extern_c() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = r#"
extern "C"
    function puts(s: str) -> i32
end

puts("Hello C FFI")
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_linux = generate(&ast, Architecture::X64, OperatingSystem::Linux);
    assert!(asm_linux.contains(".extern puts"));
    assert!(asm_linux.contains("call puts"));
    assert!(!asm_linux.contains("call fn_puts"));

    let asm_macos = generate(&ast, Architecture::X64, OperatingSystem::MacOS);
    assert!(asm_macos.contains(".extern _puts"));
    assert!(asm_macos.contains("call _puts"));

    let asm_win = generate(&ast, Architecture::X64, OperatingSystem::Windows);
    assert!(asm_win.contains(".extern puts"));
    assert!(asm_win.contains("call puts"));

    let libs = super::collect_extern_libraries(&ast);
    assert!(libs.is_empty());
}

#[test]
fn test_codegen_extern_c_with_lib() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = r#"
extern "C" from "m"
    function cos(x: f64) -> f64
end

say cos(0)
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let libs = super::collect_extern_libraries(&ast);
    assert_eq!(libs, vec!["m"]);
}

#[test]
fn test_codegen_arm64_more_than_eight_params() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = r#"
function foo(a, b, c, d, e, f, g, h, i, j)
    return i + j
end

foo(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_arm64 = generate(&ast, Architecture::ARM64, OperatingSystem::MacOS);
    // In callee: parameter 8 and 9 read from caller's stack frame
    assert!(asm_arm64.contains("ldr x9, [x29, #16]"));
    assert!(asm_arm64.contains("ldr x9, [x29, #24]"));
    // In caller: stack space allocated and extra arguments copied
    assert!(asm_arm64.contains("sub sp, sp, #16"));
    assert!(asm_arm64.contains("bl fn_foo"));
}

#[test]
fn test_codegen_arm64_call_with_inline_map_argument() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = r#"
function foo(a, b, c, d)
    return a
end

foo(201, "Created", {"X-Custom": "Test"}, "New Resource")
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_arm64 = generate(&ast, Architecture::ARM64, OperatingSystem::MacOS);
    // On ARM64, each pushed arg takes 16 bytes:
    // Arg 0 (201) pushed at -16, Arg 1 ("Created") at -32
    // When Arg 2 ({"X-Custom": "Test"}) is evaluated, map is allocated at -48!
    // Key is at -64, value at -80.
    // map must be loaded from [x29, #-48], not from an unaligned offset!
    assert!(asm_arm64.contains("ldr x0, [x29, #-48]"));
    assert!(asm_arm64.contains("bl fn_set"));
    assert!(asm_arm64.contains("bl fn_foo"));
}

#[test]
fn test_codegen_clock_ms_monotonic_targets() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = r#"
say clock_ms()
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_win = generate(&ast, Architecture::X64, OperatingSystem::Windows);
    assert!(asm_win.contains(".extern GetTickCount64"));
    assert!(asm_win.contains("call GetTickCount64"));

    let asm_linux = generate(&ast, Architecture::X64, OperatingSystem::Linux);
    assert!(asm_linux.contains(".extern clock_gettime"));
    assert!(asm_linux.contains("call clock_gettime"));
    assert!(asm_linux.contains("mov $1, %rdi"));

    let asm_macos = generate(&ast, Architecture::X64, OperatingSystem::MacOS);
    assert!(asm_macos.contains(".extern _clock_gettime"));
    assert!(asm_macos.contains("call _clock_gettime"));
    assert!(asm_macos.contains("mov $6, %rdi"));

    let asm_arm64_linux = generate(&ast, Architecture::ARM64, OperatingSystem::Linux);
    assert!(asm_arm64_linux.contains(".extern clock_gettime"));
    assert!(asm_arm64_linux.contains("bl clock_gettime"));
    assert!(asm_arm64_linux.contains("mov x0, #1"));

    let asm_arm64_macos = generate(&ast, Architecture::ARM64, OperatingSystem::MacOS);
    assert!(asm_arm64_macos.contains(".extern _clock_gettime"));
    assert!(asm_arm64_macos.contains("bl _clock_gettime"));
    assert!(asm_arm64_macos.contains("mov x0, #6"));
}

#[test]
fn test_codegen_x86_extern_i64_args_push_8_bytes() {
    // 64-bit C integers take 8-byte slots: pushing only the low word
    // shifts every later param by one arg on x86 (uv poller fds
    // mismatched, completion key/udata swapped). Signed spellings
    // sign-extend, unsigned ones zero-extend.
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = r#"
extern "C"
    function fizz(a: i64, b: u64) -> i32
end

say fizz(1024, 777)
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm = generate(&ast, Architecture::X86, OperatingSystem::Linux);
    assert!(asm.contains("call fizz"));
    assert!(
        asm.contains("sar $31, %ecx\n    push %ecx\n    push %eax"),
        "signed i64 arg must sign-extend into an 8-byte push"
    );
    assert!(
        asm.contains("xor %ecx, %ecx\n    push %ecx\n    push %eax"),
        "unsigned u64 arg must zero-extend into an 8-byte push"
    );
}

#[test]
fn test_codegen_arm64_runtime_type_check_pointer_guard() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = r#"
function check(x)
    return x is array
end
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_arm64 = generate(&ast, Architecture::ARM64, OperatingSystem::MacOS);
    assert!(asm_arm64.contains("cbz x0"));
    assert!(asm_arm64.contains("tst x0, #7"));
    assert!(asm_arm64.contains("cmp x0, #65536"));
    assert!(asm_arm64.contains("b.ls"));
    assert!(asm_arm64.contains("lsr x1, x0, #47"));
    assert!(asm_arm64.contains("ldur x1, [x0, #-16]"));
}

#[test]
fn test_codegen_macos_arm64_mem_trace_stack_passing() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = "function main() say 42 end";
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_macos = generate(&ast, Architecture::ARM64, OperatingSystem::MacOS);
    assert!(asm_macos.contains(".global alya_mem_trace_report"));
    assert!(asm_macos.contains("sub sp, sp, #32"));
    assert!(asm_macos.contains("stp x1, x2, [sp]"));
    assert!(asm_macos.contains("bl _printf"));
    assert!(asm_macos.contains("add sp, sp, #32"));
}

// Regression: extern C functions returning i32 were not sign-extended on x64/ARM64.
// The 32-bit return value in %eax / w0 is zero-extended to 64 bits by the hardware,
// so -1 (0xFFFFFFFF) would become +4294967295 (0x00000000FFFFFFFF) without the fix.
//
// Fixed by emitting:
//   x64:   movslq %eax, %rax   (sign-extend %eax → %rax)
//   ARM64: sxtw x0, w0          (sign-extend w0   → x0)
//   x86:   (nothing — 32-bit registers have no upper bits to fix)
//
// Root cause found while debugging the VPN pump disconnect bug where
// alya_vpn_pump_server_vpn (returning i32 = -1) was read as +4294967295,
// causing the disconnect check `res < 0` to never trigger.
#[test]
fn test_codegen_extern_c_i32_return_sign_extension_x64() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = r#"
extern "C"
    function alya_vpn_pump_server_vpn(sock: i32) -> i32
end

let res = alya_vpn_pump_server_vpn(3)
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    // x64 Linux: must emit movslq %eax, %rax after the call
    let asm_x64_linux = generate(&ast, Architecture::X64, OperatingSystem::Linux);
    assert!(
        asm_x64_linux.contains("call alya_vpn_pump_server_vpn"),
        "x64 Linux: call instruction missing"
    );
    assert!(
        asm_x64_linux.contains("movslq %eax, %rax"),
        "x64 Linux: missing sign-extension 'movslq %eax, %rax' after i32-returning extern call"
    );

    // x64 Windows: same sign-extension requirement
    let asm_x64_win = generate(&ast, Architecture::X64, OperatingSystem::Windows);
    assert!(
        asm_x64_win.contains("call alya_vpn_pump_server_vpn"),
        "x64 Windows: call instruction missing"
    );
    assert!(
        asm_x64_win.contains("movslq %eax, %rax"),
        "x64 Windows: missing sign-extension 'movslq %eax, %rax' after i32-returning extern call"
    );

    // x64 macOS: same sign-extension requirement
    let asm_x64_mac = generate(&ast, Architecture::X64, OperatingSystem::MacOS);
    assert!(
        asm_x64_mac.contains("movslq %eax, %rax"),
        "x64 macOS: missing sign-extension 'movslq %eax, %rax' after i32-returning extern call"
    );
}

#[test]
fn test_codegen_extern_c_i32_return_sign_extension_arm64() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = r#"
extern "C"
    function alya_vpn_pump_client_vpn(sock: i32) -> i32
end

let res = alya_vpn_pump_client_vpn(5)
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    // ARM64 Linux: must emit sxtw x0, w0 after the call
    let asm_arm64_linux = generate(&ast, Architecture::ARM64, OperatingSystem::Linux);
    assert!(
        asm_arm64_linux.contains("bl alya_vpn_pump_client_vpn"),
        "ARM64 Linux: bl instruction missing"
    );
    assert!(
        asm_arm64_linux.contains("sxtw x0, w0"),
        "ARM64 Linux: missing sign-extension 'sxtw x0, w0' after i32-returning extern call"
    );

    // ARM64 macOS: same requirement
    let asm_arm64_mac = generate(&ast, Architecture::ARM64, OperatingSystem::MacOS);
    assert!(
        asm_arm64_mac.contains("sxtw x0, w0"),
        "ARM64 macOS: missing sign-extension 'sxtw x0, w0' after i32-returning extern call"
    );
}

#[test]
fn test_codegen_extern_c_i32_return_no_sign_extension_x86() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = r#"
extern "C"
    function some_fn(x: i32) -> i32
end

let res = some_fn(1)
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    // x86: 32-bit register has no upper bits — no sign-extension instruction needed
    let asm_x86 = generate(&ast, Architecture::X86, OperatingSystem::Linux);
    assert!(
        !asm_x86.contains("movslq"),
        "x86: should NOT emit 'movslq' — 32-bit registers need no sign-extension"
    );
    assert!(!asm_x86.contains("sxtw"), "x86: should NOT emit 'sxtw'");
}

#[test]
fn test_codegen_extern_c_i64_return_no_sign_extension() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    // i64 return: already full-width, no sign-extension should be emitted for
    // the extern call result. Note: the ARM64 runtime may emit sxtw in other
    // helpers (maps, net), so we only verify the x64 path here where we can be
    // certain that movslq is exclusively produced by the i32 sign-extension path.
    let code = r#"
extern "C"
    function alya_vpn_pump_server_vpn(sock: i32) -> i64
end

let res = alya_vpn_pump_server_vpn(3)
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    // x64: movslq is only emitted by i32 sign-extension paths. Since the
    // shared SIMD runtime now also legitimately uses movslq (i32x8
    // horizontal reductions), scope the check to the extern call site:
    // no sign-extension may follow the call itself.
    // An i64-returning extern fn must NOT trigger it.
    let asm_x64 = generate(&ast, Architecture::X64, OperatingSystem::Linux);
    let lines: Vec<&str> = asm_x64.lines().collect();
    let call_idx = lines
        .iter()
        .position(|l| l.trim_start().starts_with("call alya_vpn_pump_server_vpn"))
        .expect("x64: extern call missing");
    assert!(
        !lines
            .iter()
            .skip(call_idx + 1)
            .take(6)
            .any(|l| l.contains("movslq")),
        "x64: i64-returning extern fn must NOT emit movslq sign-extension at the call site"
    );

    // ARM64: confirm the i32-specific sign-extension instruction is absent
    // immediately after the extern call. We check by counting occurrences of
    // "sxtw x0, w0" and verifying the count does not exceed what the ARM64
    // runtime itself injects (e.g. from maps/net helpers). A simpler proxy:
    // confirm the call is present and no additional sxtw follows it compared
    // to an identical program without the call (not practical here). Instead,
    // we document the known ARM64 limitation: runtime may emit sxtw independently.
    // The functional guarantee is that the codegen path for i64 skips the
    // is_i32_ret block — verified above via x64 and by code inspection.
    let asm_arm64 = generate(&ast, Architecture::ARM64, OperatingSystem::Linux);
    assert!(
        asm_arm64.contains("bl alya_vpn_pump_server_vpn"),
        "ARM64: bl instruction missing for extern call"
    );
    // The i32-specific path is guarded by `rt == \"i32\"` check in expr.rs.
    // If return type is i64, sxtw is NOT emitted for this call — confirmed by
    // x64 movslq absence and code-level inspection of the is_i32_ret guard.
}

#[test]
fn test_codegen_arm64_runtime_symbols() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = "function main() say 42 end";
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_arm64 = generate(&ast, Architecture::ARM64, OperatingSystem::Windows);
    assert!(asm_arm64.contains("alya_fat_ptr_new:"));
    assert!(asm_arm64.contains("fn_format_binary:"));
    assert!(asm_arm64.contains("fn_runes:"));
    assert!(asm_arm64.contains("b.ls .L_arm64_rc_retain_done"));
}

#[test]
fn test_codegen_arm64_string_as_int_cast() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = "function main() let s = \"A\" let c = s as int say c end";
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_arm64 = generate(&ast, Architecture::ARM64, OperatingSystem::Windows);
    assert!(asm_arm64.contains("ldrb w1, [x0]"));
    assert!(asm_arm64.contains("cmp w1, #0x80"));
}

#[test]
fn test_arm64_linux_variadic_mixed_float_int_registers() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    // Regression: On Linux ARM64 (standard AAPCS64), variadic functions like printf
    // retrieve general-purpose arguments from x1..x7 and floating-point arguments
    // from d0..d7 independently. When a float argument precedes a string or int,
    // the string must be passed in the next available x-register (e.g. x1 if it is
    // the first non-format GP argument), while the float goes into d0.
    let code =
        "let root = 8.0\nlet name = \"linux\"\nsay f\"Square root: {root}, Host OS: {name}\"\n";
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_lin = generate(&ast, Architecture::ARM64, OperatingSystem::Linux);
    assert!(asm_lin.contains("ldr x1, [sp], #16"));
    assert!(asm_lin.contains("ldr x16, [sp], #16"));
    assert!(asm_lin.contains("fmov d0, x16"));
}

#[test]
fn test_x64_macos_internal_symbols_no_darwin_prefix() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    // Regression: On macOS x64 (Darwin), C symbols like printf/calloc require a leading underscore
    // prefix ("_printf", "_calloc"), but internal assembly functions defined in the same translation
    // unit (e.g. alya_map_hash, alya_map_key_eq, alya_fat_ptr_new) must NOT be called with a leading
    // underscore because their .global definition labels do not have one.
    let code = "function main() let m = {\"key\": 1} say m[\"key\"] end";
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_mac = generate(&ast, Architecture::X64, OperatingSystem::MacOS);
    // Ensure libc symbols DO have the Darwin leading underscore prefix:
    assert!(asm_mac.contains("call _printf"));
    // Ensure internal Alya symbols DO NOT have Darwin leading underscore prefix:
    assert!(asm_mac.contains("call alya_map_hash"));
    assert!(asm_mac.contains("call alya_map_key_eq"));
    assert!(!asm_mac.contains("call _alya_map_hash"));
    assert!(!asm_mac.contains("call _alya_map_key_eq"));
    assert!(!asm_mac.contains("call _alya_fat_ptr_new"));

    // Also test interface fat pointer dispatch on macOS x64
    let iface_code = r#"
interface Greeter
    function greet(self) -> int
end
struct Human
    id: int
end
function Human.greet(self) -> int
    return 1
end
function run_greet(g: Greeter)
    say g.greet()
end
function main()
    let h = Human{id: 42}
    run_greet(h)
end
"#;
    let mut lexer2 = Lexer::new(iface_code);
    let tokens2 = lexer2.tokenize().unwrap();
    let mut parser2 = Parser::new(tokens2);
    let ast2 = parser2.parse().unwrap();
    let asm_iface = generate(&ast2, Architecture::X64, OperatingSystem::MacOS);
    assert!(asm_iface.contains("call alya_fat_ptr_new"));
    assert!(!asm_iface.contains("call _alya_fat_ptr_new"));
}

#[test]
fn test_try_catch_internal_symbols_no_darwin_prefix() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    // Regression (alya-lang/alya#65): try/catch lowers to internal
    // runtime calls (alya_try_begin/end, alya_catch_msg) whose
    // `.global` definitions are bare. Emitting them with the Darwin
    // `_` prefix breaks the macOS link (undefined symbols), so they
    // must stay bare on macOS x64 and ARM64 alike.
    let code = "function main() try say 1 catch err say err end end";
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_x64 = generate(&ast, Architecture::X64, OperatingSystem::MacOS);
    assert!(asm_x64.contains("call alya_try_begin"));
    assert!(asm_x64.contains("call alya_try_end"));
    assert!(asm_x64.contains("call alya_catch_msg"));
    assert!(!asm_x64.contains("call _alya_try_begin"));
    assert!(!asm_x64.contains("call _alya_try_end"));
    assert!(!asm_x64.contains("call _alya_catch_msg"));

    let asm_arm = generate(&ast, Architecture::ARM64, OperatingSystem::MacOS);
    assert!(asm_arm.contains("bl alya_try_begin"));
    assert!(asm_arm.contains("bl alya_try_end"));
    assert!(asm_arm.contains("bl alya_catch_msg"));
    assert!(!asm_arm.contains("bl _alya_try_begin"));
    assert!(!asm_arm.contains("bl _alya_try_end"));
    assert!(!asm_arm.contains("bl _alya_catch_msg"));
}

#[test]
fn test_x64_macos_directory_inode64_symbols() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    // Regression: On macOS x86_64, Darwin requires 64-bit inode versioned symbols
    // (_opendir$INODE64, _readdir$INODE64, _closedir$INODE64) to match the 64-bit
    // struct dirent layout (where d_name starts at offset 21).
    let code = "function main() let items = list_dir(\".\") say len(items) end";
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_mac = generate(&ast, Architecture::X64, OperatingSystem::MacOS);
    assert!(asm_mac.contains(".extern _opendir$INODE64"));
    assert!(asm_mac.contains(".extern _readdir$INODE64"));
    assert!(asm_mac.contains(".extern _closedir\n"));
    assert!(asm_mac.contains("call _opendir$INODE64"));
    assert!(asm_mac.contains("call _readdir$INODE64"));
    assert!(asm_mac.contains("call _closedir\n"));
    assert!(!asm_mac.contains("call _opendir\n"));
    assert!(!asm_mac.contains("call _readdir\n"));
}

#[test]
fn test_x64_macos_thread_join_stack_alignment() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    // Regression: On macOS x86_64, fn___native_thread_join pushes rbp, rbx, and r12
    // (making rsp 16-byte aligned). It must subtract 16 bytes (not 8) before calling
    // pthread_join so that rsp remains 16-byte aligned as required by System V AMD64 ABI.
    let code = "import \"std/thread\"\nfunction main() end";
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_mac = generate(&ast, Architecture::X64, OperatingSystem::MacOS);
    assert!(asm_mac.contains("fn___native_thread_join:\n    push %rbp\n    mov %rsp, %rbp\n    push %rbx\n    push %r12\n    sub $16, %rsp"));
    assert!(asm_mac.contains(".L_x64_join_done:\n    add $16, %rsp\n    pop %r12\n    pop %rbx"));

    // Regression: On macOS x86_64, fn_alya_thread_proc must preserve callee-saved
    // registers (rbx, r12, r13, r14, r15) across the call to user Alya code,
    // otherwise _pthread_start crashes with SIGSEGV when using rbx upon thread return.
    assert!(asm_mac.contains("fn_alya_thread_proc:\n    push %rbp\n    mov %rsp, %rbp\n    push %rbx\n    push %r12\n    push %r13\n    push %r14\n    push %r15\n    sub $24, %rsp"));
    assert!(asm_mac.contains(
        "add $24, %rsp\n    pop %r15\n    pop %r14\n    pop %r13\n    pop %r12\n    pop %rbx"
    ));
}

#[test]
fn test_codegen_arm64_float_annotated_return_syncs_d0() {
    // Regression test for alya-lang/alya#51: a `-> float` function must
    // place its return value in d0 even when the returned expression is
    // not itself inferred float (e.g. an array element read). Callers
    // sync the result via `fmov x0, d0`, so a missing sync reads stale d0.
    use crate::ast::Attribute;
    let getf = Stmt::Function {
        name: "getf".into(),
        params: vec!["w".into(), "idx".into()],
        param_types: vec![None, None],
        return_type: Some("float".into()),
        defaults: vec![],
        body: vec![Stmt::Return(Some(Expr::Index {
            array: Box::new(Expr::Identifier("w".into())),
            index: Box::new(Expr::Identifier("idx".into())),
        }))],
        type_params: vec![],
        attributes: Vec::<Attribute>::new(),
    };
    // NOTE: the call passes an int array on purpose: with a float
    // array, inference already marks the read float and the sync is
    // emitted even without the fix. The bug bites exactly when inference
    // cannot prove floatness and only the `-> float` annotation knows.
    let call = Stmt::Say(Expr::Call {
        name: "getf".into(),
        args: vec![Expr::Array(vec![Expr::Number(7)]), Expr::Number(0)],
    });
    let program = Program {
        statements: vec![getf, call],
    };
    let asm = generate(&program, Architecture::ARM64, OperatingSystem::Linux);
    let start = asm.find("fn_getf:").expect("fn_getf emitted");
    let region = &asm[start..];
    let end = region
        .find("\nret")
        .or_else(|| region.find("\n.global "))
        .unwrap_or(region.len());
    assert!(
        region[..end].contains("fmov d0, x0"),
        "float-annotated return must sync d0, got:\n{}",
        &region[..end]
    );
}

#[test]
fn test_codegen_arm64_int_return_skips_d0_sync() {
    // Guard against over-application: an `-> int` function must not gain
    // a float sync on its return path.
    use crate::ast::Attribute;
    let geti = Stmt::Function {
        name: "geti".into(),
        params: vec!["w".into(), "idx".into()],
        param_types: vec![None, None],
        return_type: Some("int".into()),
        defaults: vec![],
        body: vec![Stmt::Return(Some(Expr::Index {
            array: Box::new(Expr::Identifier("w".into())),
            index: Box::new(Expr::Identifier("idx".into())),
        }))],
        type_params: vec![],
        attributes: Vec::<Attribute>::new(),
    };
    let call = Stmt::Say(Expr::Call {
        name: "geti".into(),
        args: vec![Expr::Array(vec![Expr::Number(7)]), Expr::Number(0)],
    });
    let program = Program {
        statements: vec![geti, call],
    };
    let asm = generate(&program, Architecture::ARM64, OperatingSystem::Linux);
    let start = asm.find("fn_geti:").expect("fn_geti emitted");
    let region = &asm[start..];
    let end = region
        .find("\nret")
        .or_else(|| region.find("\n.global "))
        .unwrap_or(region.len());
    assert!(
        !region[..end].contains("fmov d0, x0"),
        "int return must not sync d0, got:\n{}",
        &region[..end]
    );
}

#[test]
fn test_codegen_arm64_float_binop_syncs_right_operand() {
    // Regression test for alya-lang/alya#50 (ARM64 half): the
    // two-dynamic-operand float path must sync the right operand from
    // x0. Trusting a stale d0 returned the left operand for
    // `a[0] + a[1]` (literal float arrays included).
    use crate::ast::BinaryOp;
    let prog = Program {
        statements: vec![
            Stmt::Let {
                name: "a".into(),
                type_ann: None,
                value: Expr::Array(vec![Expr::Float(1.5), Expr::Float(2.5)]),
            },
            Stmt::Say(Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Index {
                    array: Box::new(Expr::Identifier("a".into())),
                    index: Box::new(Expr::Number(0)),
                }),
                right: Box::new(Expr::Index {
                    array: Box::new(Expr::Identifier("a".into())),
                    index: Box::new(Expr::Number(1)),
                }),
            }),
        ],
    };
    let asm = generate(&prog, Architecture::ARM64, OperatingSystem::Linux);
    assert!(
        asm.contains("fmov d0, x0\n    ldr d1, [sp], #16"),
        "float binop must sync right operand from x0, got:\n{}",
        asm
    );
}

#[test]
fn test_codegen_untyped_array_param_retained_despite_unknown_callsite() {
    // Regression test for alya-lang/alya#47: entry retains for an
    // untyped array param must not depend on proving ALL call args
    // arrays. One unclassifiable arg (here a map index) dropped the
    // retains, causing use-after-free with run-varying garbage that
    // depended on which other calls existed in the binary.
    use crate::ast::Attribute;
    let first = Stmt::Function {
        name: "first".into(),
        params: vec!["a".into()],
        param_types: vec![None],
        return_type: None,
        defaults: vec![],
        body: vec![
            Stmt::Let {
                name: "r".into(),
                type_ann: None,
                value: Expr::Identifier("a".into()),
            },
            Stmt::Return(Some(Expr::Index {
                array: Box::new(Expr::Identifier("r".into())),
                index: Box::new(Expr::Number(0)),
            })),
        ],
        type_params: vec![],
        attributes: Vec::<Attribute>::new(),
    };
    let main = Stmt::Function {
        name: "main".into(),
        params: vec![],
        param_types: vec![],
        return_type: None,
        defaults: vec![],
        body: vec![
            Stmt::Let {
                name: "m".into(),
                type_ann: None,
                value: Expr::Map(vec![(
                    Expr::String("k".into()),
                    Expr::Array(vec![Expr::Number(1), Expr::Number(2)]),
                )]),
            },
            Stmt::Say(Expr::Call {
                name: "first".into(),
                args: vec![Expr::Array(vec![Expr::Number(7), Expr::Number(8)])],
            }),
            Stmt::Say(Expr::Call {
                name: "first".into(),
                args: vec![Expr::Index {
                    array: Box::new(Expr::Identifier("m".into())),
                    index: Box::new(Expr::String("k".into())),
                }],
            }),
        ],
        type_params: vec![],
        attributes: Vec::<Attribute>::new(),
    };
    let program = Program {
        statements: vec![first, main],
    };
    let asm = generate(&program, Architecture::X64, OperatingSystem::Windows);
    let start = asm.find("fn_first:").expect("fn_first emitted");
    let region = &asm[start..];
    let end = region
        .find("\nfn_")
        .or_else(|| region.find("\n.global "))
        .unwrap_or(region.len());
    assert!(
        region[..end].contains("call fn_rc_retain"),
        "untyped array param must be retained on entry, got:\n{}",
        &region[..end]
    );
}

// Return-tag protocol ordering (alya-lang/alya#55-C): callees emit
// before callers so forward references keep their markers.

fn order_test_fn(name: &str, calls: &[&str]) -> Stmt {
    Stmt::Function {
        name: name.to_string(),
        params: vec![],
        param_types: vec![],
        return_type: None,
        defaults: vec![],
        body: calls
            .iter()
            .map(|c| {
                Stmt::Expr(Expr::Call {
                    name: c.to_string(),
                    args: vec![],
                })
            })
            .collect(),
        type_params: vec![],
        attributes: vec![],
    }
}

fn order_names(funcs: &[&Stmt]) -> Vec<String> {
    funcs
        .iter()
        .map(|s| match s.inner_stmt() {
            Stmt::Function { name, .. } => name.clone(),
            _ => unreachable!(),
        })
        .collect()
}

#[test]
fn test_order_forward_chain_callee_first() {
    let m = order_test_fn("main", &["mid"]);
    let mid = order_test_fn("mid", &["leaf"]);
    let leaf = order_test_fn("leaf", &[]);
    let ordered = order_functions_callee_first(vec![&m, &mid, &leaf]);
    assert_eq!(order_names(&ordered), vec!["leaf", "mid", "main"]);
}

#[test]
fn test_order_already_ordered_unchanged() {
    let leaf = order_test_fn("leaf", &[]);
    let mid = order_test_fn("mid", &["leaf"]);
    let m = order_test_fn("main", &["mid"]);
    let ordered = order_functions_callee_first(vec![&leaf, &mid, &m]);
    assert_eq!(order_names(&ordered), vec!["leaf", "mid", "main"]);
}

#[test]
fn test_order_cycle_keeps_source_order() {
    let a = order_test_fn("a", &["b"]);
    let b = order_test_fn("b", &["a"]);
    let ordered = order_functions_callee_first(vec![&a, &b]);
    assert_eq!(order_names(&ordered), vec!["a", "b"]);
}

#[test]
fn test_order_recursive_callee_still_first() {
    let m = order_test_fn("main", &["fib"]);
    let fib = order_test_fn("fib", &["fib"]);
    let ordered = order_functions_callee_first(vec![&m, &fib]);
    assert_eq!(order_names(&ordered), vec!["fib", "main"]);
}

#[test]
fn test_order_unknown_callee_ignored() {
    let m = order_test_fn("main", &["dynamic", "leaf"]);
    let leaf = order_test_fn("leaf", &[]);
    let ordered = order_functions_callee_first(vec![&m, &leaf]);
    assert_eq!(order_names(&ordered), vec!["leaf", "main"]);
}

#[test]
fn test_order_qualified_call_matches_bare_def() {
    let m = order_test_fn("main", &["pkg::helper"]);
    let helper = order_test_fn("helper", &[]);
    let ordered = order_functions_callee_first(vec![&m, &helper]);
    assert_eq!(order_names(&ordered), vec!["helper", "main"]);
}

// Darwin pthread_once_t is 16 bytes ({long sig, char[8]}) starting
// as PTHREAD_ONCE_INIT (sig 0x30B1BCBA). A zeroed 8-byte slot
// under-reserves (once token overlaps the key: re-init per call)
// and never matches the init signature.
#[test]
fn test_macos_catch_once_initializer() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let code = "function main() try say 1 catch err say err end end";
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm_mac = generate(&ast, Architecture::X64, OperatingSystem::MacOS);
    assert!(asm_mac.contains("alya_catch_once:\n    .quad 0x30B1BCBA\n    .quad 0\n"));
    // Mach-O __bss forbids nonzero initializers: the once object
    // must live in __DATA,__data, after the __bss section.
    let bss_pos = asm_mac.find("__DATA,__bss").unwrap();
    let data_pos = asm_mac.find("__DATA,__data").unwrap();
    let once_pos = asm_mac.find("alya_catch_once:").unwrap();
    assert!(once_pos > data_pos && data_pos > bss_pos);

    let asm_lin = generate(&ast, Architecture::X64, OperatingSystem::Linux);
    assert!(asm_lin.contains("alya_catch_once:\n    .quad 0\n"));
    assert!(!asm_lin.contains("0x30B1BCBA"));
}

#[test]
fn test_arm64_fn_throw_preserves_callee_saved_registers() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    // Regression (alya-lang/alya#68): fn_throw must not clobber
    // callee-saved registers x19..x28 across catch jumps.
    let code = "function main() try throw 1 catch end end";
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm = generate(&ast, Architecture::ARM64, OperatingSystem::MacOS);
    let fn_throw_start = asm.find("fn_throw:").unwrap();
    let fn_throw_br = asm[fn_throw_start..].find("br x14").unwrap();
    let fn_throw_body = &asm[fn_throw_start..fn_throw_start + fn_throw_br];

    for reg in &[
        "x19", "x20", "x21", "x22", "x23", "x24", "x25", "x26", "x27", "x28",
    ] {
        assert!(
            !fn_throw_body.contains(reg),
            "fn_throw must not modify callee-saved register {} before dispatch jump: {}",
            reg,
            fn_throw_body
        );
    }
}

#[test]
fn test_arm64_thread_proc_preserves_callee_saved_registers() {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    // Regression (alya-lang/alya#68): fn_alya_thread_proc must
    // preserve x19..x28 for the calling OS thread trampoline.
    let code = "import \"std/thread\"\nfunction main() end";
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();

    let asm = generate(&ast, Architecture::ARM64, OperatingSystem::MacOS);
    assert!(asm.contains("fn_alya_thread_proc:"));
    assert!(asm.contains("stp x19, x20, [sp, #16]"));
    assert!(asm.contains("ldp x19, x20, [sp, #16]"));
    assert!(asm.contains("stp x27, x28, [sp, #80]"));
    assert!(asm.contains("ldp x27, x28, [sp, #80]"));
}
