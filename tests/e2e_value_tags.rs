mod common;
use common::*;

// Issue alya-lang/alya#39 — value-kind tag conformance matrix.
//
// Every scenario that must behave a certain way lives here: exact
// values, compile-time rejections (via spec/negative fixtures, NOT
// here), and runtime strict errors. Structural limits (opaque `is`
// on floats, unknown-tag writes, forward-ref/recursion misses) are
// documented in code, never locked in.

// ---- A. say reads ----

#[test]
fn test_tags_say_literal_mixed_array() {
    let code = r#"
function main()
    let m = [10, 0.5, "s"]
    say m[0]
    say m[1]
    say m[2]
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "10\n0.5\ns\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_say_push_int_array() {
    let code = r#"
function main()
    let m = []
    m.push(1)
    m.push(2)
    say m[0]
    say m[1]
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n2\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_say_push_float_array() {
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
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n1.5\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_say_push_mixed_array() {
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
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n0.5\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_say_map_literal() {
    let code = r#"
function main()
    let m = {"i": 1, "f": 0.5, "s": "x"}
    say m["i"]
    say m["f"]
    say m["s"]
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n0.5\nx\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_say_map_indexassign() {
    let code = r#"
function main()
    let m = {}
    m["i"] = 1
    m["f"] = 0.5
    m["s"] = "x"
    say m["i"]
    say m["f"]
    say m["s"]
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n0.5\nx\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_say_map_variable_key() {
    let code = r#"
function main()
    let m = {}
    m["f"] = 0.5
    let k = "f"
    if clock_ms() == 0
        k = "x"
    end
    say m[k]
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_say_untyped_param_reads() {
    let code = r#"
function show(a)
    say a[0]
    say a[1]
    say a[2]
end

function main()
    let m = []
    m.push(1)
    m.push(0.5)
    m.push("s")
    show(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n0.5\ns\n", "Got: {}", output);
    }
}

// ---- B. `is` checks ----

#[test]
fn test_tags_is_proven_array() {
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
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "i\nf\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_is_proven_map() {
    let code = r#"
function main()
    let m = {"i": 1, "f": 0.5, "s": "x"}
    say when m["i"]
        is int => "i"
        else => "?"
    end
    say when m["f"]
        is float => "f"
        else => "?"
    end
    say when m["s"]
        is string => "s"
        else => "?"
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "i\nf\ns\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_is_dynamic_param() {
    let code = r#"
function probe(a)
    say when a[0]
        is float => "f"
        else => "i"
    end
    say when a[1]
        is string => "s"
        else => "?"
    end
end

function main()
    let m = []
    m.push(0.5)
    m.push("s")
    probe(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "f\ns\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_kindof_int_and_string_reads() {
    // NOTE: kind() of a *float* read is structurally opaque (issue
    // #39: runtime classification cannot see floats) and is NOT
    // covered here. Ints and strings classify exactly.
    let code = r#"
function kind(v) -> string
    return when v
        is string => "s"
        is int => "i"
        is float => "f"
        else => "?"
    end
end

function main()
    let m = []
    m.push(1)
    m.push("s")
    say kind(m[0])
    say kind(m[1])
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "i\ns\n", "Got: {}", output);
    }
}

// ---- C. arithmetic over dynamics ----

#[test]
fn test_tags_arith_int_dynamics() {
    // All-int dynamics keep integer semantics (incl. int division).
    let code = r#"
function calc(a)
    say a[0] + a[1]
    say a[0] - a[1]
    say a[0] * a[1]
    say a[0] / a[1]
    say a[0] % a[1]
end

function main()
    let m = []
    m.push(7)
    m.push(2)
    calc(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "9\n5\n14\n3\n1\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_arith_static_float_promotion() {
    // An explicit float side promotes (no strict error).
    let code = r#"
function calc(a)
    say a[0] + 1.0
    say a[0] * 2.0
end

function main()
    let m = []
    m.push(1)
    calc(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "2\n2\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_arith_mixed_dynamics_strict_error_add() {
    let code = r#"
function calc(a)
    return a[0] + a[1]
end

function main()
    let m = []
    m.push(1)
    m.push(0.5)
    say calc(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
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
fn test_tags_arith_mixed_dynamics_strict_error_div() {
    let code = r#"
function calc(a)
    return a[0] / a[1]
end

function main()
    let m = []
    m.push(1)
    m.push(0.5)
    say calc(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
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
fn test_tags_arith_float_float_dynamics_strict_error() {
    // Any float tag in a non-statically-float op traps (printing the
    // f64 result would need value tags, rejected in Phase 3).
    let code = r#"
function calc(a)
    return a[0] + a[1]
end

function main()
    let m = []
    m.push(0.5)
    m.push(1.5)
    say calc(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
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

// ---- D. bitwise over dynamics ----

#[test]
fn test_tags_bitwise_int_dynamics() {
    let code = r#"
function calc(a)
    say a[0] & a[1]
    say a[0] | a[1]
    say a[0] ^ a[1]
    say a[0] << 1
    say a[0] >> 1
end

function main()
    let m = []
    m.push(6)
    m.push(3)
    calc(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "2\n7\n5\n12\n3\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_bitwise_mixed_dynamics_strict_error() {
    let code = r#"
function calc(a)
    return a[0] & a[1]
end

function main()
    let m = []
    m.push(1)
    m.push(0.5)
    say calc(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
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

// ---- E. comparisons ----

#[test]
fn test_tags_cmp_int_dynamics() {
    let code = r#"
function cmp(a)
    say a[0] == a[1]
    say a[0] != a[1]
    say a[0] < a[1]
    say a[0] > a[1]
    say a[0] <= a[1]
    say a[0] >= a[1]
end

function main()
    let m = []
    m.push(2)
    m.push(3)
    cmp(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0\n1\n1\n0\n1\n0\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_cmp_mixed_dynamics_strict_error_eq() {
    let code = r#"
function cmp(a)
    return a[0] == 1
end

function main()
    let m = []
    m.push(1.0)
    say cmp(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
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
fn test_tags_cmp_mixed_dynamics_strict_error_lt() {
    let code = r#"
function cmp(a)
    return a[0] < a[1]
end

function main()
    let m = []
    m.push(1)
    m.push(0.5)
    say cmp(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
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
fn test_tags_cmp_static_float_promotion() {
    // Explicit float side: promotion, correct boolean.
    let code = r#"
function cmp(a)
    return a[0] == 1.0
end

function main()
    let m = []
    m.push(1.0)
    say cmp(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_condition_int_dynamics() {
    let code = r#"
function check(a)
    if a[0] < a[1]
        say "lt"
    end
    while a[0] > 0
        say "pos"
        break
    end
end

function main()
    let m = []
    m.push(1)
    m.push(2)
    check(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "lt\npos\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_condition_mixed_dynamics_strict_error() {
    let code = r#"
function check(a)
    if a[0] < 2
        say "lt"
    end
end

function main()
    let m = []
    m.push(0.5)
    check(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
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

// ---- F. conversions ----

#[test]
fn test_tags_str_of_reads() {
    let code = r#"
function main()
    let m = []
    m.push(1)
    m.push(0.5)
    m.push("s")
    say str(m[0])
    say str(m[1])
    say str(m[2])
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n0.5\ns\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_str_of_dynamic_param_reads() {
    let code = r#"
function show(a)
    say str(a[0])
    say str(a[1])
end

function main()
    let m = []
    m.push(1)
    m.push(0.5)
    show(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n0.5\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_float_of_int_read() {
    let code = r#"
function main()
    let m = []
    m.push(1)
    say float(m[0])
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_int_of_float_read() {
    let code = r#"
function main()
    let m = []
    m.push(2.5)
    say int(m[0])
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "2\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_explicit_conversion_allows_mixed() {
    // Explicit conversions exempt the static mixed error.
    let code = r#"
function main()
    let m = []
    m.push(1)
    m.push(0.5)
    say float(m[0]) + m[1]
    say int(m[1]) + m[0]
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1.5\n1\n", "Got: {}", output);
    }
}

// ---- G. interpolation ----

#[test]
fn test_tags_interpolated_mixed_reads() {
    let code = r#"
function main()
    let m = []
    m.push(1)
    m.push(0.5)
    m.push("s")
    say "v={m[0]}"
    say "v={m[1]}"
    say "v={m[2]}"
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "v=1\nv=0.5\nv=s\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_interpolated_static_parts() {
    let code = r#"
function main()
    let x = 41
    let y = 0.5
    say "x={x} y={y}!"
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "x=41 y=0.5!\n", "Got: {}", output);
    }
}

// ---- H. loops ----

#[test]
fn test_tags_for_over_int_array() {
    let code = r#"
function main()
    let m = [1, 2, 3]
    for v in m
        say v
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n2\n3\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_for_over_float_array() {
    let code = r#"
function main()
    let m = [0.5, 1.5]
    for v in m
        say v
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n1.5\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_for_over_mixed_array_truncates() {
    // Documented semantics: mixed elements convert to the loop
    // variable's static (int) type instead of reinterpreting bits.
    let code = r#"
function main()
    let m = []
    m.push(1)
    m.push(0.5)
    for v in m
        say v
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n0\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_for_over_map() {
    let code = r#"
function main()
    let m = {"a": "x", "b": 1}
    for k, v in m
        say k
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        let mut got: Vec<&str> = output.lines().collect();
        got.sort_unstable();
        assert_eq!(got, vec!["a", "b"], "Got: {}", output);
    }
}

#[test]
fn test_tags_for_over_mixed_map_values() {
    // Mixed maps demote the value to Number; say classifies per value.
    let code = r#"
function main()
    let m = {"a": "x", "b": 1}
    for k, v in m
        say v
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        let mut got: Vec<&str> = output.lines().collect();
        got.sort_unstable();
        assert_eq!(got, vec!["1", "x"], "Got: {}", output);
    }
}

#[test]
fn test_tags_while_with_dynamic_bound() {
    let code = r#"
function count(a)
    let n = 0
    let i = 0
    while i < a[0]
        n = n + 1
        i = i + 1
    end
    say n
end

function main()
    let m = []
    m.push(3)
    count(m)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "3\n", "Got: {}", output);
    }
}

// ---- I. ternary results ----

#[test]
fn test_tags_ternary_dynamic_float_arm() {
    let code = r#"
function main()
    let m = []
    m.push(0.5)
    let t = clock_ms()
    say if t != 0 then m[0] else 1
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_ternary_int_arm_wins() {
    let code = r#"
function main()
    let m = []
    m.push(0.5)
    let t = clock_ms()
    say if t == 0 then m[0] else 1
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_ternary_string_arm() {
    let code = r#"
function main()
    let m = []
    m.push(1)
    let t = clock_ms()
    say if t != 0 then "s" else m[0]
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "s\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_ternary_nested() {
    let code = r#"
function main()
    let m = []
    m.push(0.5)
    let t = clock_ms()
    say if t != 0 then (if t != 0 then m[0] else 1) else 2
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_ternary_in_arithmetic_strict_error() {
    // A float-valued ternary in integer arithmetic traps. (Single-push
    // float arrays would be *proven* float and promote instead; the
    // mixed array keeps the arm dynamically unknown.)
    let code = r#"
function main()
    let m = []
    m.push(1)
    m.push(0.5)
    let t = clock_ms()
    say (if t != 0 then m[1] else 1) + 1
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
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

// ---- J. call results (return-tag protocol) ----

#[test]
fn test_tags_call_float_result() {
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
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_call_int_result() {
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
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_call_result_in_arithmetic_strict_error() {
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
    if let Some((code, output)) = run_alya_code_full(code) {
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
fn test_tags_call_result_str_float() {
    // str()/float() see through qualified calls.
    let code = r#"
function pick(a)
    let t = clock_ms()
    return if t != 0 then a[0] else 1
end

function main()
    let m = []
    m.push(0.5)
    say str(pick(m))
    say float(pick(m))
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n0.5\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_call_result_comparison_promotion() {
    // Explicit float side promotes across the call boundary.
    let code = r#"
function pick(a)
    let t = clock_ms()
    return if t != 0 then a[0] else 1
end

function main()
    let m = []
    m.push(1.0)
    say pick(m) == 1.0
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_call_literal_returns() {
    // Literal returns materialize kinds (int and float).
    let code = r#"
function five()
    return 5
end

function half()
    return 0.5
end

function main()
    say five()
    say half()
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "5\n0.5\n", "Got: {}", output);
    }
}

// ---- K. misc ----

#[test]
fn test_tags_null_coalesce_dynamic() {
    let code = r#"
function main()
    let m = []
    m.push(0.5)
    let x = m[0] ?? 1
    say x
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n", "Got: {}", output);
    }
}

#[test]
fn test_tags_string_concat_dynamics() {
    let code = r#"
function cat(a)
    return a[0] + "x"
end

function main()
    let m = []
    m.push(1)
    say cat(m)
    let n = []
    n.push(0.5)
    say cat(n)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "1x\n0.5x\n", "Got: {}", output);
    }
}

// NOTE (structural, not locked in): `is float` on a truly opaque
// value, unknown-tag writes, forward-reference/recursion misses, and
// NaN-boxing/pointer-tags (rejected) stay out of this matrix.
