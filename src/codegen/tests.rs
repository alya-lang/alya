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
    assert!(asm_arm64.contains("b.lo"));
    assert!(asm_arm64.contains("lsr x1, x0, #47"));
    assert!(asm_arm64.contains("ldur x1, [x0, #-16]"));
}
