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
for i in 0..(len(arr) - 1)
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
for i in 1..15
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
