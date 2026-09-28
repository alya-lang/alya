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
