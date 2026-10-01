mod common;
use common::*;

// alya-lang/alya#39 Phase 1, x86 tier: 32-bit slots carry 8-byte values
// with sidecar/entry kind tags (RED: x86 stores stale %eax for floats and
// reads everything raw). Each test skips where the toolchain lacks -m32.

#[test]
fn test_x86_array_float_roundtrip() {
    let code = r#"
function main()
    let m = []
    m.push(0.5)
    m.push(1.5)
    say m[0]
    say m[1]
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n1.5\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_mixed_array_say() {
    let code = r#"
function main()
    let m = []
    m.push(1)
    m.push(0.5)
    say m[0]
    say m[1]
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n0.5\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_map_float_read_and_loop() {
    let code = r#"
function main()
    let m = {}
    m["a"] = 0.5
    say m["a"]
    for k, v in m
        say v
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n0\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_kind_dispatch() {
    // Direct `when ... is` on index reads dispatches on the slot tag.
    // (Through a generic function param the tag is opaque on every
    // arch — same boundary as x64; tested via when-in-main here.)
    let code = r#"
function main()
    let m = []
    m.push(1)
    m.push(0.5)
    say when m[0]
        is float => "f"
        else => "i"
    end
    say when m[1]
        is float => "f"
        else => "i"
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "i\nf\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_mixed_string_int() {
    let code = r#"
function main()
    let m = []
    m.push("s")
    m.push(1)
    say m[0]
    say m[1]
    for k, v in m
        say k
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "s\n1\n0\n1\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_inline_literal_map_records_tags() {
    // Same as the x64 inline-literal tag test: literal construction
    // records entry tags; loop values convert, direct reads dispatch.
    let code = r#"
function main()
    say ({"a": 0.5})["a"]
    for k, v in {"a": 0.5}
        say v
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n0\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_interpolated_mixed_read_dispatches_on_tag() {
    // x86 twin: interpolation of tag-carrying reads routes through
    // str() instead of printing raw bits.
    let code = r#"
function main()
    let m = []
    m.push(1)
    m.push(0.5)
    say "v={m[1]}"
    say "v={m[0]}"
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "v=0.5\nv=1\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_untyped_param_mixed_strict_error() {
    // x86 twin of the strict dynamic check: float tags on dynamic
    // reads trap instead of computing on raw bits.
    let code = r#"
function sum2(a)
    return a[0] + a[1]
end

function main()
    let m = []
    m.push(1)
    m.push(0.5)
    say sum2(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_ne!(
            code, 0,
            "Expected a runtime mixed-type error, got: {}",
            output
        );
        assert!(
            output.contains("mixed int/float arithmetic"),
            "Got: {}",
            output
        );
    }
}

#[test]
fn test_x86_untyped_param_int_stays_int() {
    // No false positive on x86 either.
    let code = r#"
function sum2(a)
    return a[0] + a[1]
end

function main()
    let m = []
    m.push(1)
    m.push(2)
    say sum2(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "3\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_ternary_dynamic_arm_carries_tag() {
    // x86 twin: same-scope ternary with a tag-carrying arm prints
    // the taken arm's value via tag dispatch.
    let code = r#"
function main()
    let m = []
    m.push(0.5)
    let t = clock_ms()
    say if t != 0 then m[0] else 1
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_ternary_int_arm_wins() {
    // No false positive on x86 either.
    let code = r#"
function main()
    let m = []
    m.push(0.5)
    let t = clock_ms()
    say if t == 0 then m[0] else 1
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_call_boundary_tag_protocol_float() {
    // x86 twin of the return-tag protocol: float arm prints exactly.
    let code = r#"
function pick(a)
    let t = clock_ms()
    return if t != 0 then a[0] else 1
end

function main()
    let m = []
    m.push(0.5)
    say pick(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_call_boundary_tag_protocol() {
    // x86 twin of the return-tag protocol (int arm; float values on
    // x86 additionally need the pre-existing float-return value path).
    let code = r#"
function pick(a)
    let t = clock_ms()
    return if t == 0 then a[0] else 1
end

function main()
    let m = []
    m.push(0.5)
    say pick(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_call_boundary_strict_via_call() {
    // Strict checks see through qualified calls on x86 too.
    let code = r#"
function pick(a)
    let t = clock_ms()
    return if t != 0 then a[0] else 1
end

function main()
    let m = []
    m.push(0.5)
    say pick(m) + 1
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_ne!(
            code, 0,
            "Expected a runtime mixed-type error, got: {}",
            output
        );
        assert!(
            output.contains("mixed int/float arithmetic"),
            "Got: {}",
            output
        );
    }
}

#[test]
fn test_x86_map_float_let_spill() {
    // Inferred-`any` locals bound from a map float read must keep the
    // full double: the `let` used to push only `%eax` (the lo word),
    // so `2.5` (0x4004000000000000) read back as `0`.
    let code = r#"
function main()
    let m = { "f1": 2.5 }
    let v = m["f1"]
    say v
    say v + 0.5
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "2.5\n3\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_map_float_arg_to_float_param() {
    // A map float read passed to a `float` param must arrive as an
    // 8-byte double: the caller pushed only `%eax`, so the callee
    // read lo-word plus adjacent stack garbage.
    let code = r#"
function ident_f(x: float) -> float
    return x
end

function main()
    let m = { "f1": 2.5 }
    say ident_f(m["f1"])
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "2.5\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_map_float_dynamic_key_arg_and_layout() {
    // Dynamic keys carry no marker (the cache package pattern), and a
    // widened first arg must not shift the slots after it.
    let code = r#"
function ident_f(x: float) -> float
    return x
end

function getf(values: map, k: string) -> float
    return ident_f(values[k])
end

function mix(x: float, y: int) -> float
    return x + y
end

function main()
    let m = { "f1": 2.5, "i1": 7 }
    say getf(m, "f1")
    say mix(m["f1"], m["i1"])
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "2.5\n9.5\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_map_int_arg_to_int_param_unchanged() {
    // Widened index pushes stay value-safe for 4-byte callees: an int
    // param still reads the same lo word, and the caller cleanup stays
    // balanced (a mismatch here would crash, not just misprint).
    let code = r#"
function ident_i(x: int) -> int
    return x
end

function main()
    let m = { "i1": 7 }
    say ident_i(m["i1"])
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "7\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_say_int_consts() {
    // Regression test for alya-lang/alya#56: `%lld` reads 8 bytes, so
    // constant ints must push hi+lo (not a single 4-byte word).
    let code = r#"
enum HttpStatus
    Ok = 200
    NotFound = 404
end
say HttpStatus.Ok
say HttpStatus::NotFound
say 200
say 404
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "200\n404\n200\n404\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_integer_literal_precision() {
    // Regression test for alya-lang/alya#57: 64-bit literals print
    // exactly through the hi/lo constant path.
    let code = r#"
say 2305843009213693951
say 4611686018427387903
say 9223372036854775807
say 0xFF
say 1_000_000
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(
            output, "2305843009213693951\n4611686018427387903\n9223372036854775807\n255\n1000000\n",
            "Got: {}",
            output
        );
    }
}

#[test]
fn test_x86_str_to_int_rodata() {
    // Regression test for alya-lang/alya#58: the rodata classifier must
    // not clobber the sign flag, or every parsed int comes out negated.
    let code = r#"
say to_int("36")
say to_int("-36")
let kind = "i"
let raw = "36"
let decoded = when kind
    is "i" => to_int(raw)
    else => raw
end
say "decoded: " + str(decoded)
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "36\n-36\ndecoded: 36\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_say_string_array() {
    // Regression test for alya-lang/alya#59: fn_join must stride 8-byte
    // slots (not 4), or multi-element string arrays crash/mismatch.
    let code = r#"
function pick(lang: string) -> array
    return when lang
        is "a" => ["x", "y"]
        else => ["p", "q"]
    end
end
function main()
    let r = pick("a")
    say r
    say r[0]
    say r[1]
    say ["x", "y"]
end
main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "[x, y]\nx\ny\n[x, y]\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_extern_i64_args_sign_extended() {
    // i64 extern args must arrive as full 8-byte slots: pushing only
    // the low word shifts every later C param by one arg (uv poller
    // fds mismatched, completion key/udata swapped). llabs takes and
    // returns long long, so a wrong hi word comes back wrong: -5 must
    // arrive sign-extended (hi 0xFFFFFFFF) and return 5, while a zero
    // hi word would come back as -5.
    let code = r#"
extern "C"
    function llabs(n: i64) -> i64
end

function main()
    say llabs(-5)
    say llabs(777)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "5\n777\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_issue70_shared_helper_is_checks() {
    // alya-lang/alya#70 is arch-independent (inference-level): shared
    // helpers testing `is array` / `is map` must not fold when callers
    // pass different static kinds.
    let code = r#"
function is_list(v) -> int
    if v is array
        return 1
    end
    return 0
end

function is_map_obj(v) -> int
    if v is map
        return 1
    end
    return 0
end

function helper(v, level: int, indent_size: int) -> string
    return "H"
end

function disp(val, level: int, indent_size: int) -> string
    if val is null
        return "null"
    end
    if is_list(val) == 1
        return helper(val, level, indent_size)
    elif is_map_obj(val) == 1
        return "MAP"
    else
        return "OTHER"
    end
end

function main()
    say disp({ "a": 1 }, 0, 2)
    say disp([1], 0, 2)
    say disp(42, 0, 2)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "MAP\nH\nOTHER\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_issue71_typeof_degraded_to_int() {
    // alya-lang/alya#71 on x86: 32-bit slots cannot tell a heap
    // address from float low-bits, so the cascade skips the header
    // checks instead of risking the read. Arrays/maps through dynamic
    // `typeof` report "int" (the old constant fallback) while strings
    // still dispatch at runtime. Locked in as documented behavior, not
    // as a goal: a future tagged representation should close it.
    let code = r#"
function kind_of(v) -> string
    return typeof(v)
end

function main()
    say kind_of({ "a": 1 })
    say kind_of([1])
    say kind_of(42)
    say kind_of("hi")
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "int\nint\nint\nstring\n", "Got: {}", output);
    }
}

#[test]
fn test_x86_struct_float_field_as_call_arg() {
    // Struct float fields live in uniform 8-byte slots and the field
    // read leaves the double in `%xmm0` (like map/array gets), so a
    // field read passed to a float param must push 8 bytes, not 4.
    // (math package: `t_approx(s.re, ...)` read back shifted garbage.)
    let code = r#"
struct Cxy
    re: float,
    im: float
end

function t_approx(a: float, b: float, eps: float) -> int
    let d = a - b
    if d < 0.0
        d = 0.0 - d
    end
    if d <= eps
        return 1
    end
    return 0
end

function main()
    let s = Cxy { re: 4.0, im: -2.0 }
    say t_approx(s.re, 4.0, 0.000001)
    say t_approx(s.im, -2.0, 0.000001)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_x86(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n1\n", "Got: {}", output);
    }
}
