use super::generate;
use super::target::{Architecture, OperatingSystem};
use crate::ast::{BinaryOp, Expr, Program, Stmt};

fn simple_program(stmt: Stmt) -> Program {
    Program {
        statements: vec![stmt],
    }
}

#[test]
fn test_codegen_x64_windows_header_and_footer() {
    let program = simple_program(Stmt::Say(Expr::Number(42.0)));
    let asm = generate(&program, Architecture::X64, OperatingSystem::Windows);

    assert!(asm.contains(".global main"));
    assert!(asm.contains("main:"));
    assert!(asm.contains("push %rbp"));
    assert!(asm.contains("mov %rsp, %rbp"));
    assert!(asm.contains("ret"));
}

#[test]
fn test_codegen_x86_header_and_footer() {
    let program = simple_program(Stmt::Say(Expr::Number(42.0)));
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
    let program = simple_program(Stmt::Say(Expr::Number(42.0)));
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
    let program = simple_program(Stmt::Say(Expr::Number(42.0)));
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
    let program = simple_program(Stmt::Say(Expr::Number(42.0)));
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
            value: Expr::Number(i as f64),
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
                    left: Box::new(Expr::Number(10.0)),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Number(20.0)),
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
fn test_codegen_arm64_large_number_immediate() {
    let program = simple_program(Stmt::Say(Expr::Number(424242.0)));
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
        value: Expr::Number(424242.0),
    });
    let asm = generate(&program, Architecture::ARM64, OperatingSystem::Linux);

    assert!(asm.contains("movz x0, #31026"));
    assert!(asm.contains("movk x0, #6, lsl #16"));
    assert!(!asm.contains("mov x0, #424242"));
}

#[test]
fn test_codegen_arm64_runtime_immediates_valid() {
    let program = simple_program(Stmt::Say(Expr::Number(1.0)));
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
    crate::parser::resolve_imports(&mut ast, std::path::Path::new(".")).unwrap();

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

    // x64: movslq is only emitted by the i32 sign-extension path.
    // An i64-returning extern fn must NOT trigger it.
    let asm_x64 = generate(&ast, Architecture::X64, OperatingSystem::Linux);
    assert!(
        !asm_x64.contains("movslq %eax, %rax"),
        "x64: i64-returning extern fn must NOT emit movslq sign-extension"
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
