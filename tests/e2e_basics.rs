mod common;
use common::*;

#[test]
fn test_e2e_arithmetic() {
    let code = r#"
say 10 + 20
say 15 * 3
say 100 - 45
say 50 / 2
say 17 % 5
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "30\n45\n55\n25\n2\n");
    }
}

#[test]
fn test_e2e_variables_and_arithmetic() {
    let code = r#"
let a = 10
let b = 25
let sum = a + b
say sum
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "35\n");
    }
}

#[test]
fn test_e2e_conditionals() {
    let code = r#"
let x = 42
if x > 50
    say "gt"
else
    say "le"
end
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "le\n");
    }
}

#[test]
fn test_e2e_loops() {
    let code = r#"
for i in 1..4
    say i
end

let j = 10
while j < 13
    say j
    j = j + 1
end
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "1\n2\n3\n4\n10\n11\n12\n");
    }
}

#[test]
fn test_e2e_else_if_and_elif() {
    let code = r#"
let score = 85
if score >= 90
    say "A"
else if score >= 80
    say "B"
else
    say "C"
end

let val = 5
if val == 1
    say "one"
elif val == 5
    say "five"
else
    say "other"
end
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "B\nfive\n");
    }
}

#[test]
fn test_e2e_compound_assignment() {
    let code = r#"
let x = 10
x += 5
x -= 2
x *= 3
x /= 2
say x
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "19\n"); // ((10 + 5) - 2) * 3 / 2 = 39 / 2 = 19
    }
}

#[test]
fn test_e2e_slash_comments_and_logical_symbols() {
    let code = r#"
// This is a C-style comment
/* Multi-line
   comment */
let a = 10
let b = 20
if (a < 15) && (b == 20)
    say "and works"
end

if (a == 99) || (b > 10)
    say "or works"
end

if !(a == 99)
    say "not works"
end
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "and works\nor works\nnot works\n");
    }
}

#[test]
fn test_e2e_repeat_loop() {
    let code = r#"
let loops = 0
repeat
    loops += 1
    if loops >= 3
        break
    end
end
say loops
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "3\n");
    }
}

#[test]
fn test_e2e_break_and_continue() {
    let code = r#"
let sum = 0
for i in 1..10
    if i % 2 == 0
        continue
    end
    if i > 6
        break
    end
    sum += i
end
say sum

let w = 0
let w_sum = 0
while w < 10
    w += 1
    if w == 2
        continue
    end
    if w == 5
        break
    end
    w_sum += w
end
say w_sum
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        // for loop: 1 + 3 + 5 = 9
        // while loop: w=1 (sum=1), w=2 (continue), w=3 (sum=4), w=4 (sum=8), w=5 (break) -> 8
        assert_eq!(output, "9\n8\n");
    }
}

#[test]
fn test_e2e_multi_variable_let() {
    let code = r#"
let idx, val = 0
say idx
say val

let x, y = 10, 20
say x
say y

let a, b, c = 100
say a
say b
say c

let first, second = "alpha", "beta"
say first
say second

let m, n = 5 + 3, 4 * 6
say m
say n
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "0\n0\n10\n20\n100\n100\n100\nalpha\nbeta\n8\n24\n");
    }
}

#[test]
fn test_e2e_when_enhanced() {
    let code = r#"
# 1. Multi-statement arm and optional then
let x = 2
let count = 0
when x
    is 1
        say "one"
        count += 10
    is 2 then
        say "two"
        count += 20
        count += 5
    else
        say "other"
        count += 99
end
say count

# 2. Multi-value and Range matching
let status = 201
when status
    is 200, 201, 204 then say "Success"
    is 400..499 then say "Client error"
    is >= 500 then say "Server error"
    else say "Unknown"
end

let score = 85
when score
    is 90..100
        say "Grade: A"
    is 80..89
        say "Grade: B"
    is < 60
        say "Grade: F"
    else
        say "Grade: C"
end

# 3. Side-effect safety (evaluating expression only once)
function get_val()
    say "evaluating subject"
    return 3
end

when get_val()
    is 1 then say "one"
    is 2 then say "two"
    is 3 then say "three"
    else say "none"
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "two\n25\nSuccess\nGrade: B\nevaluating subject\nthree\n"
        );
    }
}

#[test]
fn test_e2e_bitwise_operators() {
    let code = r#"
# Test bitwise AND, OR, XOR, Shift Left, Shift Right, Bitwise NOT
let a = 12
let b = 10

say a & b
say a | b
say a ^ b
say 1 << 4
say 32 >> 2
say ~0

# Test compound assignment
let x = 7
x &= 3
say x

x |= 8
say x

x ^= 2
say x

x <<= 2
say x

x >>= 3
say x
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "8\n14\n6\n16\n8\n-1\n3\n11\n9\n36\n4\n");
    }
}

#[test]
fn test_e2e_null_literal() {
    let code = r#"
let a = null
let b = nil

say a
say b
say null

if a == null
    say "a is null"
end

if b == nil
    say "b is nil"
end

if a == b
    say "null equals nil"
end

let c = 42
if c != null
    say "c is not null"
end

say "value: {a}"
say str(a)

let x = null
x = 100
say x
x = null
say x
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "null\nnull\nnull\na is null\nb is nil\nnull equals nil\nc is not null\nvalue: null\nnull\n100\nnull\n");
    }
}

#[test]
fn test_e2e_ternary_and_inline_if() {
    let code = r#"
let age = 20
let status = age >= 18 ? "Adult" : "Minor"
say status

let num = -5
let sign = if num > 0 then "positive" else "non-positive"
say sign

let a = 10
let b = 20
let max_val = a > b ? a : b
say max_val

let min_val = if a < b then a else b
say min_val

let nested = age > 10 ? (age > 18 ? "Adult" : "Teen") : "Child"
say nested
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "Adult\nnon-positive\n20\n10\nAdult\n");
    }
}

#[test]
fn test_e2e_multiline_strings() {
    let code = r#"
let triple = """
Line 1
Line 2
"""
say triple

let raw = `First line
Second line`
say raw

let item = "Alya"
let templ = """
Hello, {item}!
Welcome!
"""
say templ
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(
            output,
            "Line 1\nLine 2\n\nFirst line\nSecond line\nHello, Alya!\nWelcome!\n\n"
        );
    }
}

#[test]
fn test_e2e_null_coalescing() {
    let code = r#"
let custom_port = null
let port = custom_port ?? 8080
say port

let active_port = 3000
let port2 = active_port ?? 8080
say port2

let user = null
let name = user ?? "Guest"
say name

let logged_in = "Alice"
let name2 = logged_in ?? "Guest"
say name2

let a = null
let b = null
let c = "Fallback"
let chosen = a ?? b ?? c
say chosen

let x = null ?? 42
say x

let direct = "Direct" ?? "Ignored"
say direct
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "8080\n3000\nGuest\nAlice\nFallback\n42\nDirect\n");
    }
}

#[test]
fn test_e2e_short_circuiting() {
    let code = r#"
// 1. Guard check for null object should not crash (segfault)
let obj = null
if obj != null and obj[0] == 1
    say "unreachable"
else
    say "null guard passed"
end

// 2. Guard check for empty array
let arr = []
if len(arr) > 0 and arr[0] == 10
    say "unreachable"
else
    say "array guard passed"
end

// 3. Or short-circuiting: second operand should not evaluate if first is true
let y = 1
if y == 1 or y / 0 == 0
    say "or short-circuit passed"
else
    say "unreachable"
end

// 4. Short-circuit in expressions
let flag_and_false = 0 and 1
let flag_and_true = 1 and 1
let flag_or_true = 1 or 0
let flag_or_false = 0 or 0
say flag_and_false
say flag_and_true
say flag_or_true
say flag_or_false
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(
            output,
            "null guard passed\narray guard passed\nor short-circuit passed\n0\n1\n1\n0\n"
        );
    }
}
