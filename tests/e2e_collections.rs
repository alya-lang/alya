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
