mod common;
use common::*;

#[test]
fn test_e2e_arrays() {
    let code = r#"
let arr = [10, 20, 30]
say len(arr)
say arr[0]
say arr[1]
say arr[2]
arr[1] = 99
arr[0] += 5
say arr
let sum = 0
for i in 0..len(arr)
    sum += arr[i]
end
say sum
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "3\n10\n20\n30\n[15, 99, 30]\n144\n");
    }
}

#[test]
fn test_e2e_dynamic_arrays() {
    let code = r#"
let arr = []
say len(arr)
arr.push(10)
arr.push(20)
push(arr, 30)
say len(arr)
say arr
let last = arr.pop()
say last
say len(arr)
say arr
let last2 = pop(arr)
say last2
say len(arr)
say arr

// Test growth beyond initial capacity 8
let big = []
for i in 1..=15
    big.push(i * 2)
end
say len(big)
say big[0]
say big[14]

// Test pop on empty array caught by try/catch
let empty = []
try
    empty.pop()
    say "should not reach"
catch err
    say "caught empty pop: " + err
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "0\n3\n[10, 20, 30]\n30\n2\n[10, 20]\n20\n1\n[10]\n15\n2\n30\ncaught empty pop: index out of bounds\n"
        );
    }
}

#[test]
fn test_e2e_structs() {
    let code = r#"
struct Point
    x
    y
end

let p = Point { x: 10, y: 20 }
say p.x
say p.y
say p

p.x = 99
p.y += 5
say p.x
say p.y

let p2 = Point(1, 2)
say p2.x
say p2.y

function translate(pt, dx, dy)
    pt.x += dx
    pt.y += dy
    return pt
end

let p3 = translate(p2, 10, 20)
say p3.x
say p3.y

say "Formatted point: ({p.x}, {p.y})"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "10\n20\nPoint { x: 10, y: 20 }\n99\n25\n1\n2\n11\n22\nFormatted point: (99, 25)\n"
        );
    }
}

#[test]
fn test_e2e_structs_advanced() {
    let code = r#"
struct Person
    name
    age
    score
end

let alice = Person { name: "Alice", age: 30, score: 95.5 }
say alice.name
say alice.age
say alice.score
say "Student: {alice.name}, Age: {alice.age}, Score: {alice.score}"

struct Vector3
    x
    y
    z
end

let v1 = Vector3(1.0, 2.0, 3.5)
let v2 = Vector3(0.5, 1.5, 0.5)
let dot = v1.x * v2.x + v1.y * v2.y + v1.z * v2.z
say "Dot product: {dot}"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "Alice\n30\n95.5\nStudent: Alice, Age: 30, Score: 95.5\nDot product: 5.25\n"
        );
    }
}

#[test]
fn test_e2e_struct_mixed_field_kinds() {
    // One untyped field holding different literal kinds across instances
    // must not poison reads globally: no single static marker serves
    // every instance, so global markers are dropped and reads fall back
    // to runtime classification plus per-variable keys.
    // (Regression test for alya-lang/alya#15.)
    let code = r#"
struct Box
    value
end

function main()
    let a = Box { value: 1 }
    say a.value
    say str(a.value)
    let b = Box { value: "Ada" }
    say b.value
    say a.value
    say "a: " + str(a.value) + ", b: " + b.value
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "1\n1\nAda\n1\na: 1, b: Ada\n");
    }
}

#[test]
fn test_e2e_struct_mixed_field_kinds_float() {
    // Reversed direction: float first, int second. The stale float
    // marker previously printed int bits as `%g` garbage.
    // (Regression test for alya-lang/alya#15.)
    let code = r#"
struct Box
    value
end

function main()
    let a = Box { value: 3.5 }
    say a.value
    let b = Box { value: 1 }
    say b.value
    say a.value
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "3.5\n1\n3.5\n");
    }
}

#[test]
fn test_e2e_maps() {
    let code = r#"
let m = map()
say m.len()

m["foo"] = 42
m["bar"] = 100
say m["foo"]
say m["bar"]
say m["missing"]
say m.len()

m["foo"] = 99
say m["foo"]
say m.len()

say m.contains("foo")
say m.contains("missing")
say m.has("bar")

m.set("baz", 777)
say m.get("baz")

let rem = m.remove("foo")
say rem
say m.contains("foo")
say m.len()

let ks = m.keys()
say ks.len()

let vs = m.values()
say vs.len()

let sum = 0
for k in m.keys()
    sum = sum + m[k]
end
say sum

let m0 = map()
say m0

let m1 = map()
m1["answer"] = 42
say m1
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "0\n",            // m.len() initially
                "42\n",           // m["foo"]
                "100\n",          // m["bar"]
                "0\n",            // m["missing"]
                "2\n",            // m.len()
                "99\n",           // updated m["foo"]
                "2\n",            // m.len() after update
                "1\n",            // m.contains("foo")
                "0\n",            // m.contains("missing")
                "1\n",            // m.has("bar")
                "777\n",          // m.get("baz")
                "1\n",            // m.remove("foo")
                "0\n",            // m.contains("foo") after remove
                "2\n",            // m.len() after remove
                "2\n",            // ks.len()
                "2\n",            // vs.len()
                "877\n",          // sum over keys: 100 + 777
                "{}\n",           // say m0
                "{answer: 42}\n", // say m1
            )
        );
    }
}

#[test]
fn test_e2e_map_string_values() {
    let code = r#"
let m = map()
m["name"] = "Alya"
m["version"] = "1.0"
say m["name"]
say m["version"]
say "Language: " + m["name"]
m["name"] = "Alya Lang"
say m["name"]
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!("Alya\n", "1.0\n", "Language: Alya\n", "Alya Lang\n",)
        );
    }
}

#[test]
fn test_e2e_map_literals() {
    let code = r#"
let empty = {}
say empty
say empty.len()

let user = { "name": "Alya", age: 2 }
say user["name"]
say user["age"]
say user.len()

let explicit = map { "foo": 123 }
say explicit["foo"]

let nested = { "outer": { "inner": 99 } }
let inner_map = nested["outer"]
say inner_map["inner"]

say { "inline": 777 }["inline"]
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!("{}\n", "0\n", "Alya\n", "2\n", "2\n", "123\n", "99\n", "777\n",)
        );
    }
}

#[test]
fn test_e2e_struct_array_iteration_and_field_interpolation() {
    let code = r#"
struct Player
    name
    score
end

function rank_player(p)
    if p.score >= 90
        return "Master"
    elif p.score >= 75
        return "Expert"
    else
        return "Challenger"
    end
end

let team = [
    Player { name: "Alice", score: 95 },
    Player { name: "Bob", score: 82 }
]

for member in team
    let tier = rank_player(member)
    say "Player {member.name} scored {member.score} pts -> [{tier}]"
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "Player Alice scored 95 pts -> [Master]\nPlayer Bob scored 82 pts -> [Expert]\n"
        );
    }
}

#[test]
fn test_e2e_array_and_string_slicing() {
    let code = r#"
let arr = [10, 20, 30, 40, 50]

let s1 = arr[1..4]
say len(s1)
say s1[0]
say s1[1]
say s1[2]

let s2 = arr[2..]
say len(s2)
say s2[0]
say s2[1]
say s2[2]

let s3 = arr[..2]
say len(s3)
say s3[0]
say s3[1]

let s4 = arr[1:3]
say len(s4)
say s4[0]
say s4[1]

let text = "Hello, World!"
let sub1 = text[0..5]
say sub1

let sub2 = text[7..12]
say sub2

let sub3 = text[7..]
say sub3

let sub4 = text[:5]
say sub4
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "3\n20\n30\n40\n",
                "3\n30\n40\n50\n",
                "2\n10\n20\n",
                "2\n20\n30\n",
                "Hello\n",
                "World\n",
                "World!\n",
                "Hello\n"
            )
        );
    }
}

#[test]
fn test_e2e_struct_methods() {
    let code = r#"
struct Point
    x
    y
end

# 1. Instance method definition on Point
function Point.sum(self)
    return self.x + self.y
end

# 2. Instance method with extra arguments
function Point.scale(self, factor)
    return Point { x: self.x * factor, y: self.y * factor }
end

# 3. Static / Factory method on Point
function Point.create(x, y)
    return Point { x: x, y: y }
end

let p = Point.create(10, 20)
say p.sum()

let p2 = p.scale(3)
say p2.x
say p2.y
say p2.sum()

# 4. Multiple structs with identical method names (name collision check)
struct Circle
    radius
end

function Circle.area(self)
    return self.radius * self.radius * 3
end

function Circle.kind(self)
    return "Circle"
end

function Point.kind(self)
    return "Point"
end

let c = Circle { radius: 5 }
say c.area()
say c.kind()
say p.kind()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "30\n30\n60\n90\n75\nCircle\nPoint\n");
    }
}

#[test]
fn test_e2e_in_and_not_in_operator() {
    let code = r#"
# 1. Map in / not in
let cfg = { "host": "127.0.0.1", "port": 8080 }
say "host" in cfg
say "missing" in cfg
say "missing" not in cfg

if "host" in cfg
    say "has host"
end
if "port" in cfg
    say "has port"
end
if "ssl" not in cfg
    say "no ssl"
end

# 2. Array in / not in
let nums = [10, 20, 30, 40]
say 20 in nums
say 99 in nums
say 99 not in nums

if 30 in nums
    say "has 30"
end
if 55 not in nums
    say "no 55"
end

# 3. String in / not in
let text = "hello alya world"
say "alya" in text
say "xyz" in text
say "xyz" not in text

if "world" in text
    say "has world"
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "1\n0\n1\nhas host\nhas port\nno ssl\n1\n0\n1\nhas 30\nno 55\n1\n0\n1\nhas world\n"
        );
    }
}

#[test]
fn test_e2e_multiple_loop_variables() {
    let code = r#"
# 1. Array with index and value
let fruits = ["apple", "banana", "cherry"]
for i, fruit in fruits
    say i
    say fruit
end

# 2. Map with key and value
let user = { "name": "Alya", "role": "admin" }
for k, v in user
    say k
    say v
end

# 3. Map with single variable (key only)
for k in user
    say k
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        // Map iteration order is insertion / bucket order, let's verify array part and map presence
        assert!(output.contains("0\napple\n1\nbanana\n2\ncherry\n"));
        assert!(output.contains("name\nAlya"));
        assert!(output.contains("role\nadmin"));
    }
}

#[test]
fn test_e2e_destructuring_and_spread() {
    // 1. Array destructuring
    let code1 = r#"
let [a, b, ...rest] = [10, 20, 30, 40, 50]
say a
say b
say len(rest)
say rest[0]
say rest[1]
say rest[2]
"#;
    if let Some((code, output)) = run_alya_code_full(code1) {
        assert_eq!(code, 0);
        assert_eq!(output, "10\n20\n3\n30\n40\n50\n");
    }

    // 2. Map destructuring
    let code2 = r#"
let person = { "name": "Alya", "age": 5, "city": "Istanbul" }
let { name: my_name, age, city } = person
say my_name
say age
say city
"#;
    if let Some((code, output)) = run_alya_code_full(code2) {
        assert_eq!(code, 0);
        assert_eq!(output, "Alya\n5\nIstanbul\n");
    }

    // 3. Variadic function arguments
    let code3 = r#"
function calc_total(base, ...numbers)
    let sum = base
    for n in numbers
        sum += n
    end
    return sum
end

say calc_total(100, 1, 2, 3, 4)
say calc_total(50)
"#;
    if let Some((code, output)) = run_alya_code_full(code3) {
        assert_eq!(code, 0);
        assert_eq!(output, "110\n50\n");
    }

    // 4. Array spread
    let code4 = r#"
let a = [1, 2]
let b = [4, 5]
let combined = [...a, 3, ...b]
say len(combined)
for x in combined
    say x
end
"#;
    if let Some((code, output)) = run_alya_code_full(code4) {
        assert_eq!(code, 0);
        assert_eq!(output, "5\n1\n2\n3\n4\n5\n");
    }

    // 5. Map spread
    let code5 = r#"
let m1 = { "a": 1, "b": 2 }
let m2 = { "b": 20, "c": 30 }
let merged = { ...m1, ...m2, "d": 40 }
say merged["a"]
say merged["b"]
say merged["c"]
say merged["d"]
"#;
    if let Some((code, output)) = run_alya_code_full(code5) {
        assert_eq!(code, 0);
        assert_eq!(output, "1\n20\n30\n40\n");
    }
}

#[test]
fn test_e2e_push_built_float_array_reads() {
    // Regression test for alya-lang/alya#50: an array built with `push`
    // of float values must read back floats without an annotation.
    // Push-built arrays never earned the `arr_is_flt` marking that array
    // literals get, so untyped reads returned raw f64 bit patterns.
    let code = r#"
let d = []
d.push(1.5)
d.push(2.5)
say d[0]
say d[1]
say d[0] + d[1]
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "1.5\n2.5\n4\n");
    }
}

#[test]
fn test_e2e_push_built_int_array_reads_unchanged() {
    // Guard for alya-lang/alya#50: int push-built arrays must keep
    // reading ints (the new float marking must not leak onto them).
    let code = r#"
let ints = []
ints.push(10)
ints.push(20)
say ints[0]
say ints[1]
say ints[0] + ints[1]
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "10\n20\n30\n");
    }
}

#[test]
fn test_e2e_builder_returns_push_built_float_array() {
    // alya-lang/alya#50: a function that builds an array with float
    // pushes and returns it propagates floatness to the caller binding.
    let code = r#"
function build_row(a: float, b: float)
    let w = []
    w.push(a)
    w.push(b)
    return w
end

let row = build_row(1.5, 2.5)
say row[0]
say row[1]
say row[0] + row[1]
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "1.5\n2.5\n4\n");
    }
}

#[test]
fn test_e2e_mixed_push_array_reads_dispatch() {
    // alya-lang/alya#39 Phase 1: array slots carry kinds, so reads of
    // a mixed push-built array dispatch per element. Today the float
    // slot reads back raw f64 bits.
    let code = r#"
let m = []
m.push(1)
m.push(0.5)
say m[0]
say m[1]
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "1\n0.5\n");
    }
}

#[test]
fn test_e2e_mixed_push_array_is_checks() {
    // alya-lang/alya#39 Phase 1: `is int` / `is float` on reads of a
    // mixed push-built array dispatch on the slot kind.
    let code = r#"
let m = []
m.push(1)
m.push(0.5)
say m[0] is int
say m[1] is float
say m[1] is int
say m[0] is float
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "1\n1\n0\n0\n");
    }
}

#[test]
fn test_e2e_for_over_mixed_push_array_converts() {
    // alya-lang/alya#39 Phase 1: iterating a mixed push-built array
    // converts mismatched elements to the loop variable's static type
    // (truncate toward zero) instead of reinterpreting raw bits.
    let code = r#"
let m = []
m.push(1)
m.push(0.5)
for v in m
    say v
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "1\n0\n");
    }
}

#[test]
fn test_e2e_for_over_mixed_push_float_array_converts() {
    // Reverse direction: a float-typed loop variable converts int
    // elements (via an annotated float array holding ints).
    let code = r#"
let w: float[] = [1.5]
w.push(2)
for v in w
    say v
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "1.5\n2\n");
    }
}

#[test]
fn test_e2e_mixed_literal_array_reads_dispatch() {
    // alya-lang/alya#39 Phase 1: literal mixed arrays carry per-slot
    // kinds too, so reads dispatch instead of returning raw bits.
    let code = r#"
let a = [1, 0.5]
say a[0]
say a[1]
for v in a
    say v
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "1\n0.5\n1\n0\n");
    }
}

#[test]
fn test_e2e_is_string_on_tagged_reads() {
    // alya-lang/alya#39: `is string` on element reads with definite
    // non-string tags must be boolean false, not the leftover value.
    // (The tag path used to fall through with the value in the return
    // register, so any nonzero value read as "true".)
    let code = r#"
let w = [1, 2]
say w[0] is string
say w[0] is int
let r = ["a", "b"]
say r[0] is string
say r[0] is int
let m = { "k": 42 }
say m["k"] is string
say m["k"] is int
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0\n1\n1\n0\n0\n1\n", "Got: {}", output);
    }
}
#[test]
fn test_e2e_kind_dispatch_branch_returns() {
    // alya-lang/alya#39 Phase 2b: returns behind constant-foldable
    // branches classify per live branch.
    let code = r#"
function kind(v) -> string
    return when v
        is string => "s"
        is int => "i"
        is float => "f"
        else => "?"
    end
end

function pick(b, x, y)
    if b
        return x
    else
        return y
    end
end

function main()
    say kind(pick(1, 0.5, 1))
    say kind(pick(0, 0.5, 1))
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "f\ni\n", "Got: {}", output);
    }
}

#[test]
fn test_e2e_for_over_map_float_value_converts() {
    // alya-lang/alya#39 Phase 1: map loop values convert float slots
    // to the loop variable's int type instead of printing raw bits.
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
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n0\n", "Got: {}", output);
    }
}

#[test]
fn test_e2e_for_over_mixed_map_completes() {
    // alya-lang/alya#39 Phase 1: iterating a mixed map must not fault;
    // values print per the loop variable's static type.
    let code = r#"
function main()
    let m = {}
    m["a"] = "x"
    m["b"] = 1
    let n = 0
    for k, v in m
        n = n + 1
    end
    say n
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "2\n", "Got: {}", output);
    }
}

#[test]
fn test_e2e_for_over_mixed_map_prints_values() {
    // alya-lang/alya#39 Phase 1: mixed maps demote the loop value to
    // Number via the map_nonstr veto (let-literal, index-assign, and
    // inline-literal shapes); `say` classifies each value at runtime.
    // Order-independent: map iteration order is hash-defined.
    let code = r#"
function main()
    let m = {"a": "x", "b": 1}
    for k, v in m
        say v
    end
    let n = {}
    n["c"] = 2
    n["d"] = "y"
    for k, v in n
        say v
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        let mut got: Vec<&str> = output.lines().collect();
        got.sort_unstable();
        assert_eq!(got, vec!["1", "2", "x", "y"], "Got: {}", output);
    }
}

#[test]
fn test_e2e_inline_literal_map_records_tags() {
    // alya-lang/alya#39 Phase 1: inline literal construction records
    // entry tags like IndexAssign does; loop values convert instead of
    // printing raw bits, and direct reads dispatch on the tag.
    let code = r#"
function main()
    say ({"a": 0.5})["a"]
    for k, v in {"a": 0.5}
        say v
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "0.5\n0\n", "Got: {}", output);
    }
}

#[test]
fn test_e2e_untyped_param_mixed_strict_error() {
    // alya-lang/alya#39 Phase 2b: the static checker rejects
    // provably-mixed reads; a float tag on a dynamic read is the
    // runtime half of that error (previously silent bit garbage).
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
fn test_e2e_untyped_param_int_stays_int() {
    // No false positive: all-int dynamics keep integer semantics.
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
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert_eq!(output, "3\n", "Got: {}", output);
    }
}
