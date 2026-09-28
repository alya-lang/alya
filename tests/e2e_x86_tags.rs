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
