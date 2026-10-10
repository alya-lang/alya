mod common;
use common::*;

#[test]
fn test_e2e_c_ffi_abs() {
    let code = r#"
extern "C"
    function abs(n: i32) -> i32
end

say abs(-42)
say abs(100)
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "42\n100\n");
    }
}

#[test]
fn test_e2e_c_ffi_puts() {
    let code = r#"
extern "C"
    function puts(s: str) -> i32
end

puts("Hello from C FFI")
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "Hello from C FFI\n");
    }
}

#[test]
fn test_e2e_c_ffi_strlen() {
    let code = r#"
extern "C"
    function strlen(s: str) -> i32
end

say strlen("Alya FFI")
say strlen("")
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "8\n0\n");
    }
}

#[test]
fn test_e2e_c_ffi_strcmp() {
    let code = r#"
extern "C"
    function strcmp(s1: str, s2: str) -> i32
end

say strcmp("abc", "abc")
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "0\n");
    }
}

#[test]
fn test_e2e_c_ffi_getenv() {
    let code = r#"
extern "C"
    function getenv(name: str) -> str
end

let p = getenv("PATH")
if len(p) > 0
    say "path_ok"
end
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "path_ok\n");
    }
}

#[test]
fn test_e2e_c_ffi_sprintf_float_arg() {
    // alya-lang/alya#160: SysV/AAPCS64 pass floats in a separate
    // register sequence (xmm0/d0) with AL set for variadics. The
    // double must arrive intact — it read 0.000000 before the fix.
    let code = r#"
extern "C"
    function sprintf(buf: ptr, fmt: str, val: f64) -> i32
end

let buf = alloc(32)
sprintf(buf, "%f", 19.99)
say str_from_ptr(buf)
free(buf)
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "19.990000\n");
    }
}

#[test]
fn test_e2e_c_ffi_pow_float_args() {
    // Fixed-signature double calls (xmm0/xmm1 on SysV, d0/d1 on
    // AAPCS64), including an int variable converted to double.
    let code = r#"
extern "C"
    function pow(a: f64, b: f64) -> f64
end

say pow(2.0, 10.0)
let n = 2
say pow(n, 10.0)
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "1024\n1024\n");
    }
}
