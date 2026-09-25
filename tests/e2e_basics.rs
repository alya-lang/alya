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
        assert_eq!(output, "1\n2\n3\n10\n11\n12\n");
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

#[test]
fn test_e2e_enums() {
    let code = r#"
enum Status
    Pending
    Active
    Completed
    Failed
end

enum HttpStatus
    Ok = 200
    Created = 201
    NotFound = 404
end

enum LogLevel
    Debug = "DEBUG"
    Info = "INFO"
    Warn = "WARN"
end

# 1. Dot syntax & ColonColon syntax access
let s1 = Status.Pending
let s2 = Status.Active
let s3 = Status::Completed
say s1
say s2
say s3

# 2. Custom values
say HttpStatus.Ok
say HttpStatus::NotFound
say LogLevel.Info
say LogLevel::Warn

# 3. Pattern matching with when
let state = Status.Active
when state
    is Status.Pending then say "is_pending"
    is Status.Active then say "is_active"
    is Status.Completed then say "is_completed"
    else say "unknown"
end

# 4. Comparison
if HttpStatus.Created == 201
    say "created_ok"
end
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(
            output,
            "0\n1\n2\n200\n404\nINFO\nWARN\nis_active\ncreated_ok\n"
        );
    }
}

#[test]
fn test_e2e_constants() {
    let code = r#"
const PI = 3.14159
const MAX_BUFFER = 1024
const APP_TITLE = "Alya App"
const A = 10, B = 20
const DERIVED = A * 5 + B

# 1. Top-level usage
say MAX_BUFFER
say APP_TITLE
say DERIVED

# 2. Inside functions (compile-time constant inlining)
function get_buffer_limit(factor)
    const LOCAL_PADDING = 16
    return MAX_BUFFER * factor + LOCAL_PADDING
end

say get_buffer_limit(2)

# 3. In conditionals and expressions
if MAX_BUFFER > 500
    say "buffer is large"
end
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "1024\nAlya App\n70\n2064\nbuffer is large\n");
    }
}

#[test]
fn test_const_validation_errors() {
    // 1. Reassigning constant should fail
    let bad_assign = "const X = 10\nX = 20";
    let mut lexer = alya::lexer::Lexer::new(bad_assign);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = alya::parser::Parser::new(tokens);
    let res = parser.parse();
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Cannot assign to constant 'X'"));

    // 2. Redeclaring constant should fail
    let bad_redecl = "const Y = 10\nconst Y = 20";
    let mut lexer = alya::lexer::Lexer::new(bad_redecl);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = alya::parser::Parser::new(tokens);
    let res = parser.parse();
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Cannot redeclare constant 'Y'"));
}

#[test]
fn test_e2e_struct_defaults() {
    let code = r#"
struct Config
    port = 8080
    host = "localhost"
end

# 1. Default initialization
let c1 = Config {}
say c1.port
say c1.host

# 2. Partial initialization (override port)
let c2 = Config { port: 3000 }
say c2.port
say c2.host

# 3. Partial initialization (override host)
let c3 = Config { host: "127.0.0.1" }
say c3.port
say c3.host

# 4. Positional constructor with default arguments
let c4 = Config()
say c4.port
say c4.host

let c5 = Config(9090)
say c5.port
say c5.host
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(
            output,
            "8080\nlocalhost\n3000\nlocalhost\n8080\n127.0.0.1\n8080\nlocalhost\n9090\nlocalhost\n"
        );
    }
}

#[test]
fn test_e2e_multi_assign_swap() {
    let code = r#"
# 1. Simple swap
let a = 10
let b = 20
a, b = b, a
say a
say b

# 2. Multi-assignment of distinct values
let x = 0
let y = 0
x, y = 100, 200
say x
say y

# 3. 3-way rotation swap
let u = "first"
let v = "second"
let w = "third"
u, v, w = w, u, v
say u
say v
say w

# 4. Expressions with multi-assignment
x, y = x + 5, y * 2
say x
say y

# 5. Let swap
let p = 1, q = 2
let p, q = q, p
say p
say q
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(
            output,
            "20\n10\n100\n200\nthird\nfirst\nsecond\n105\n400\n2\n1\n"
        );
    }
}

#[test]
fn test_e2e_gradual_typing() {
    let code = r#"
# 1. Full type annotations
function add(a: int, b: int) -> int
    return a + b
end

# 2. String annotations
function greet(name: str) -> str
    return "Hello, " + name
end

# 3. Float annotations
function half(x: float) -> float
    return x / 2.0
end

# 4. Gradual / Partial annotations (mixed typed & untyped)
function format_pair(prefix: str, value)
    return prefix + ": " + str(value)
end

# 5. Type annotations with default values
function multiply(x: int, factor: int = 10) -> int
    return x * factor
end

# 6. Array type annotation
function count_items(items: int[]) -> int
    return len(items)
end

say add(40, 2)
say greet("Alya")
say half(15.0)
say format_pair("Result", 99)
say multiply(5)
say multiply(5, 3)
say count_items([1, 2, 3, 4])
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "42\nHello, Alya\n7.5\nResult: 99\n50\n15\n4\n");
    }
}

#[test]
fn test_e2e_optional_chaining() {
    let code = r#"
struct Address
    city
    zip
end

struct User
    name
    age
    address
end

function format_user(u)
    return "User: " + u.name
end

let u1 = User {
    name: "Alice",
    age: 30,
    address: Address {
        city: "Istanbul",
        zip: 34000
    }
}
let u2 = null

# 1. Combined with null coalescing ??
say u1?.name ?? "Anonymous"
say u2?.name ?? "Anonymous"

# 2. Numeric field access
say u1?.age ?? 0
say u2?.age ?? 0

# 3. Nested optional chaining
say u1?.address?.city ?? "Unknown City"
say u2?.address?.city ?? "Unknown City"

# 4. Optional method call
say u1?.format_user() ?? "No user"
say u2?.format_user() ?? "No user"

# 5. Optional array indexing
let arr1 = [100, 200, 300]
let arr2 = null
say arr1?.[1] ?? -1
say arr2?.[1] ?? -1

# 6. Optional array slicing
let sub1 = arr1?.[0..2]
say sub1?.[0] ?? -1
say sub1?.[1] ?? -1
let sub2 = arr2?.[0..2]
say sub2 == null ? 1 : 0
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(
            output,
            "Alice\nAnonymous\n30\n0\nIstanbul\nUnknown City\nUser: Alice\nNo user\n200\n-1\n100\n200\n1\n"
        );
    }
}

#[test]
fn test_e2e_when_expression() {
    let code = r#"
# 1. Basic when expression with =>
let val = 2
let name = when val
    is 1 => "one"
    is 2 => "two"
    else => "other"
end
say name

# 2. When expression with ranges and relational
let score = 85
let grade = when score
    is >= 90 => "A"
    is 80..89 => "B"
    is 70..79 => "C"
    else => "F"
end
say grade

# 3. Comma-separated multiple patterns & then keyword
let num = 3
let parity = when num
    is 1, 3, 5 then "odd small"
    is 2, 4, 6 then "even small"
    else then "other"
end
say parity

# 4. Directly inside say statement
let status = 404
say when status
    is 200 => "OK"
    is 404 => "Not Found"
    else => "Error"
end

# 5. Inside arithmetic expression
let bonus = 10 + (when grade
    is "A" => 50
    is "B" => 30
    else => 0
end)
say bonus
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "two\nB\nodd small\nNot Found\n40\n");
    }
}

#[test]
fn test_e2e_type_check() {
    let code = r#"
# 1. Null check
let a = null
let b = 10
say a is null
say a is not null
say b is null
say b is not null

# 2. String check
let s = "hello"
let n = 42
say s is string
say s is not string
say n is string

# 3. Number / Int check
say n is int
say n is number
say s is number

# 4. Array check
let arr = [1, 2, 3]
say arr is array
say arr is not array
say n is array

# 5. Map check
let m = { "a": 1 }
say m is map
say m is not map
say arr is map

# 6. Inside conditionals
if s is string
    say "s is indeed string"
end
if a is not null
    say "should not print"
else
    say "a is null indeed"
end
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(
            output,
            "1\n0\n0\n1\n1\n0\n0\n1\n1\n0\n1\n0\n0\n1\n0\n0\ns is indeed string\na is null indeed\n"
        );
    }
}

#[test]
fn test_e2e_explicit_type_annotations() {
    let code = r#"
# 1. Scalar types with explicit annotations
let x: int = 42
let s: string = "hello world"
let f: float = 3.14

say x
say s
say f

# 2. Arrays with explicit annotations
let nums: int[] = [1, 2, 3]
let tags: string[] = ["alpha", "beta"]

say nums[0]
say tags[1]

# 3. Struct definition with explicit field types and default values
struct User
    name: string
    age: int = 30
    tags: string[]
end

let u1 = User { name: "Alya", age: 1, tags: ["sys", "lang"] }
say u1.name
say u1.age
say u1.tags[0]

let u2: User = User("Bob", 25, ["dev"])
say u2.name
say u2.age
say u2.tags[0]

# 4. Multi-variable let with annotations
let a: int, b: string = 100, "multi"
say a
say b
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(
            output,
            "42\nhello world\n3.14\n1\nbeta\nAlya\n1\nsys\nBob\n25\ndev\n100\nmulti\n"
        );
    }
}

#[test]
fn test_e2e_guard_as_variable_name() {
    // `guard` is a contextual keyword, not reserved: it must work as a
    // plain variable, including assignment and compound assignment.
    // (Regression test for alya-lang/alya#17.)
    let code = r#"
let guard = 10
while guard > 0
    guard = guard - 1
end
say guard
guard += 5
say guard
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "0\n5\n");
    }
}

#[test]
fn test_e2e_when_mixed_arms_str() {
    // `str()` over a `when` with mixed-type arms must convert the taken
    // arm instead of eliding conversion (which produced empty output).
    // (Regression test for alya-lang/alya#14.)
    let code = r#"
let kind = "i"
let raw = "36"
let decoded = when kind
    is "i" => to_int(raw)
    else => raw
end
say "decoded: " + str(decoded)
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "decoded: 36\n");
    }
}

#[test]
fn test_e2e_method_default_arity_collision() {
    // A bare function name colliding with a method name must not steal
    // default-argument expansion when arities differ: `b.touch("a")` has
    // 2 args, so the 1-param bare `touch` is skipped and the 3-param
    // `Box__touch` default (`ttl = -1`) is filled.
    // (Regression test for alya-lang/alya#16.)
    let code = r#"
struct Box
    store: map
end

function touch(path)
    return "file:" + path
end

function Box.touch(self: Box, key, ttl: int = -1) -> int
    if ttl == -1
        return 1
    end
    return 0
end

function main()
    let b = Box { store: map() }
    say b.touch("a")
    say touch("/tmp/x")
end

main()
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "1\nfile:/tmp/x\n");
    }
}

#[test]
fn test_e2e_when_type_patterns_on_dynamic_param() {
    // `is string` on an unannotated param must discriminate at runtime:
    // a single string call site must not fold the check to constant-true
    // for every other call (previously every arm after the first was dead
    // and ints printed via `%s`). Floats through `any` still read as int
    // (indistinguishable without value tags) and are not asserted here.
    // (Regression test for alya-lang/alya#14.)
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
    say kind("Ada")
    say kind(36)
end

main()
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "s\ni\n");
    }
}

#[test]
fn test_e2e_unannotated_fn_string_result_prints() {
    // A `when` with all-string arms returns a string even without a return
    // annotation; callers must print the pointer as `%s`, not `%lld`
    // garbage. (Regression test for alya-lang/alya#14.)
    let code = r#"
function decode(kind: string)
    return when kind
        is "s" => "STR"
        is "i" => "INT"
        else => "FLT"
    end
end

function main()
    say decode("s")
    say decode("i")
    say decode("z")
end

main()
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "STR\nINT\nFLT\n");
    }
}

#[test]
fn test_e2e_when_array_branches() {
    // A `when` returning string arrays from every arm must print array
    // elements as strings (not raw pointers), including element reads.
    // (Regression test for alya-lang/alya#18a.)
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
end

main()
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "[x, y]\nx\ny\n");
    }
}

#[test]
fn test_e2e_say_variable_key_map_read() {
    // Map reads with variable (runtime) keys must print like literal-key
    // reads: statically-unknown values are classified at runtime instead
    // of printing string pointers as integers.
    // (Regression test for alya-lang/alya#18b.)
    let code = r#"
let m = map()
m["k"] = "hello"
let key = "k"
let v = m[key]
say v
say m[key]
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "hello\nhello\n");
    }
}

#[test]
fn test_e2e_map_float_roundtrip() {
    // Float values stored under literal keys round-trip through reads,
    // equality, str() and `is float` via per-key kind markers.
    // (Regression test for alya-lang/alya#15.)
    let code = r#"
let m = map()
m["pi"] = 3.5
say m["pi"]
if m["pi"] == 3.5
    say "eq ok"
end
say str(m["pi"])
if m["pi"] is float
    say "is-float ok"
end
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "3.5\neq ok\n3.5\nis-float ok\n");
    }
}

#[test]
fn test_e2e_map_rewrite_clears_markers() {
    // Rewriting a key with a proven-contradictory literal type must drop
    // stale per-key markers instead of segfaulting on `%s` over an int.
    // (Regression test for alya-lang/alya#15.)
    let code = r#"
let m = map()
m["k"] = "hello"
say m["k"]
m["k"] = 42
say m["k"]
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "hello\n42\n");
    }
}

#[test]
fn test_e2e_map_variable_key_float_tag() {
    // Variable-key reads of tagged float entries must dispatch via the
    // entry kind tag (x64 %edx from fn_get) instead of printing raw bits.
    // Covers say/str/is/== on dynamic keys. (alya-lang/alya#15.)
    let code = r#"
let m = map()
m["pi"] = 3.5
let key = "pi"
say m[key]
say str(m[key])
if m[key] is float
    say "is-float ok"
end
if m[key] == 3.5
    say "eq ok"
end
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "3.5\n3.5\nis-float ok\neq ok\n");
    }
}

#[test]
fn test_e2e_say_bss_range_integers() {
    // Numbers in the range [0x400000, 0x4400000] (such as 34992000 and 12776054)
    // fall within the static str-buf window on non-PIE Linux ELF binaries.
    // They must still be formatted as integers and not misinterpreted as strings.
    let code = r#"
let mat_result = 34992000
say mat_result

let rc4_result = 12776054
say rc4_result

fn get_val() -> int
    return 34992000
end
say get_val()
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "34992000\n12776054\n34992000\n");
    }
}
