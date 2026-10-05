mod common;
use common::*;

#[test]
fn test_e2e_string_interpolation() {
    let code = r#"
function calc(a, b)
    return a * b + 2
end

let name = "Alya"
say "Welcome to {name}!"
say "Call: {calc(3, 4)}"
say "Math: {10 + 5 * 2}"
let greeting = "   hello   "
say "Method: {greeting.trim().upper()}"
say "Escaped: {{bracket}}"
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(
            output,
            "Welcome to Alya!\nCall: 14\nMath: 20\nMethod: HELLO\nEscaped: {bracket}\n"
        );
    }
}

#[test]
fn test_e2e_string_helpers() {
    let code = r#"
let s = "  Hello, World!  "

// trim
let trimmed = s.trim()
say trimmed
say trim("   spaced out   ")

// upper and lower
let up = trimmed.upper()
say up
let low = trimmed.lower()
say low
say upper("alya language")
say lower("ALYA COMPILER")

// contains
say trimmed.contains("World")
say trimmed.contains("xyz")
say contains("abcdef", "cd")
say contains("abcdef", "gh")
say trimmed.contains("")

// substring / substr (3 args and 2 args)
let sub1 = trimmed.substring(0, 5)
say sub1
let sub2 = substr(trimmed, 7, 5)
say sub2
let sub3 = trimmed.substring(7)
say sub3
let sub4 = substr(trimmed, 7)
say sub4

// chaining and concatenation
let combo = s.trim().upper()
say combo + " - SUCCESS"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "Hello, World!\n",
                "spaced out\n",
                "HELLO, WORLD!\n",
                "hello, world!\n",
                "ALYA LANGUAGE\n",
                "alya compiler\n",
                "1\n",
                "0\n",
                "1\n",
                "0\n",
                "1\n",
                "Hello\n",
                "World\n",
                "World!\n",
                "World!\n",
                "HELLO, WORLD! - SUCCESS\n",
            )
        );
    }
}

#[test]
fn test_e2e_split_and_join() {
    let code = r#"
let fruits_str = "apple,banana,cherry"
let fruits = fruits_str.split(",")
say fruits.len()
for f in fruits
    say f
end

let joined = fruits.join(" - ")
say joined

// Function syntax
let words = split("hello world alya", " ")
say join(words, "_")

// Indexing split result
say fruits[0]
say fruits[2]

// Method chaining
let chained = "x:y:z".split(":").join("/")
say chained

// Empty delimiter (char-by-char split)
let chars = "abc".split("")
say chars.len()
for c in chars
    say c
end

// Single element & empty array join
let single = ["solo"].join(",")
say single

let empty = [].join(",")
say "empty: [{empty}]"

// Split delimiter not found
let no_match = "standalone".split(",")
say no_match.len()
say no_match[0]
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "3\n",
                "apple\n",
                "banana\n",
                "cherry\n",
                "apple - banana - cherry\n",
                "hello_world_alya\n",
                "apple\n",
                "cherry\n",
                "x/y/z\n",
                "3\n",
                "a\n",
                "b\n",
                "c\n",
                "solo\n",
                "empty: []\n",
                "1\n",
                "standalone\n",
            )
        );
    }
}

#[test]
fn test_e2e_character_tools() {
    let code = r#"
let s = "Alya 2026"
say char_at(s, 0)
say s.char_at(1)
say s[2]
say s[3]

say ord("A")
say ord("a")
say "Z".ord()
say chr(66)
say chr(98)

say is_digit("7")
say is_digit("a")
say "9".is_digit()

say is_alpha("X")
say is_alpha("_")
say is_alpha("5")
say "m".is_alpha()

say is_alnum("A")
say is_alnum("3")
say is_alnum("!")

say is_space(" ")
say is_space("\t")
say is_space("x")
say " ".is_space()

// Out of bounds safety
let out1 = char_at(s, 50)
say "oob: [{out1}]"
let out2 = s[-1]
say "neg: [{out2}]"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "A\n",
                "l\n",
                "y\n",
                "a\n",
                "65\n",
                "97\n",
                "90\n",
                "B\n",
                "b\n",
                "1\n",
                "0\n",
                "1\n",
                "1\n",
                "1\n",
                "0\n",
                "1\n",
                "1\n",
                "1\n",
                "0\n",
                "1\n",
                "1\n",
                "0\n",
                "1\n",
                "oob: []\n",
                "neg: []\n",
            )
        );
    }
}

#[test]
fn test_e2e_chr_unicode() {
    // chr() keeps single bytes for 1..255 (chr(ord(b)) round-trips, and
    // byte-oriented packages rely on it) and UTF-8 encodes 256..0x10FFFF.
    // Previously any code above 255 was dereferenced as a pointer
    // (segfault). Values above 0x10FFFF keep the legacy pointer behavior.
    let code = r#"
say chr(65)
say chr(0) == ""
say chr("AB")
say ord(chr(233))
say len(chr(233))
say len(chr(255))
say len(chr(256))
say chr(8364) == "€"
say chr(20013) == "中"
say chr(128512) == "😀"
say len(chr(127))
say len(chr(128))
say len(chr(2047))
say len(chr(2048))
say len(chr(65535))
say len(chr(65536))
say len(chr(1114111))
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "A\n", "1\n", "A\n", "233\n", "1\n", "1\n", "2\n", "1\n", "1\n", "1\n", "1\n",
                "1\n", "2\n", "3\n", "3\n", "4\n", "4\n",
            )
        );
    }
}

#[test]
fn test_e2e_char_at_unicode() {
    // B2: char_at indexes codepoints (not bytes). len() stays bytes.
    let code = r#"
say char_at("hello", 1)
say char_at("héllo", 1)
say char_at("héllo", 1) == "é"
say len(char_at("héllo", 1))
say len("héllo")
say char_count("héllo")
say char_at("a€中😀", 0)
say char_at("a€中😀", 1) == "€"
say char_at("a€中😀", 3) == "😀"
say char_at("abc", 5) == ""
say char_at("abc", -1) == ""
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!("e\n", "é\n", "1\n", "2\n", "6\n", "5\n", "a\n", "1\n", "1\n", "1\n", "1\n",)
        );
    }
}

#[test]
fn test_e2e_ord_utf8_decode() {
    // B2: ord decodes the first UTF-8 codepoint; legacy single bytes
    // (chr 1..255) round-trip.
    let code = r#"
say ord("A")
say ord(chr(8364))
say ord(chr(233))
say ord("é")
say ord("€")
say ord("中")
say ord("😀")
say ord(chr(20013))
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(output, "65\n8364\n233\n233\n8364\n20013\n128512\n20013\n");
    }
}

#[test]
fn test_e2e_runes_unicode_iteration() {
    // B2: runes() splits codepoints; for-in over strings yields rune
    // codepoints (spec ch.21 §1.4), not 1-char strings.
    let code = r#"
let r = runes("a€中😀")
say len(r)
say r[0]
say len(r[1])
say ord(r[1])
say ord(r[3])
let n = 0
let total = 0
for ch in "a€中"
    say typeof(ch)
    say chr(ch)
    total += ch
    n += 1
end
say n
say total
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            "4\na\n3\n8364\n128512\nint\na\nint\n€\nint\n中\n3\n28474\n"
        );
    }
}

#[test]
fn test_e2e_std_json_unicode_escapes() {
    // stdlib json must decode \u escapes as UTF-8 with surrogate-pair
    // combining (stdlib/json.alya). BMP singles already work via the
    // fixed fn_chr runtime; pairs and lone surrogates do not yet.
    let code = r#"
import "std/json"

say json_parse("\"\\u00E9\"")
say json_parse("\"\\u20AC\"")
say json_parse("\"\\uD83D\\uDE00\"")
say len(json_parse("\"\\uD83D\\uDE00\""))
say json_parse("\"\\uD800\"")
say len(json_parse("\"\\uD800\""))
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(output, "é\n€\n😀\n4\n�\n3\n");
    }
}

#[test]
fn test_e2e_say_letbound_json_dynamics() {
    // `let v = json_parse(...)` folds to map statically but returns any
    // JSON type at runtime. `say v` must verify before printing instead
    // of feeding scalars into alya_print_map (issue #43 follow-up).
    // Float/large-int dynamics stay a known #39 boundary and are out of
    // scope here.
    let code = r#"
import "std/json"

let s = json_parse("\"\\u20AC\"")
say s
let n = json_parse("42")
say n
let b = json_parse("true")
say b
let z = json_parse("null")
say z
let o = json_parse("{\"a\": 1}")
say o
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(output, "€\n42\n1\nnull\n{a: 1}\n");
    }
}

#[test]
fn test_e2e_str_conversion_and_concat() {
    let code = r#"
let a = 12345
say str(a)
let b = -987
say str(b)
say str(0)
say "Count: " + 42
say 100 + " percent"
let x = 50
say "Val is {x}"
say "item_" + 1 + "_part_" + 2
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "12345\n",
                "-987\n",
                "0\n",
                "Count: 42\n",
                "100 percent\n",
                "Val is 50\n",
                "item_1_part_2\n",
            )
        );
    }
}

#[test]
fn test_e2e_string_to_number_conversions() {
    let code = r#"
let s_int = "  42  "
let s_neg = "-105"
let s_flt = "3.14159"
let s_zero = "0"

let i1 = int(s_int)
let i2 = int(s_neg)
let i3 = to_int("789")
let i4 = parse_int(s_zero)

say i1
say i2
say i3
say i4
say i1 + i2

let f1 = float(s_flt)
let f2 = to_float("2.5")
let f3 = parse_float("-0.75")
let f_already = float(f1)

say f1
say f2
say f3
say f_already
say f1 + f2
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "42\n",
                "-105\n",
                "789\n",
                "0\n",
                "-63\n",
                "3.14159\n",
                "2.5\n",
                "-0.75\n",
                "3.14159\n",
                "5.64159\n",
            )
        );
    }
}

#[test]
fn test_e2e_dynamic_string_concat_and_array_indexing() {
    let code = r#"
import "std/json"

let raw = "{\"apps\": [\"chrome.exe\", \"code.exe\", \"notepad.exe\"]}"
let data = json_parse(raw)
let apps = data["apps"]

let csv = ""
let i = 0
while i < len(apps)
    if i > 0
        csv += ","
    end
    csv += apps[i]
    i += 1
end
say "csv: {csv}"

let single = apps[0]
say "app: " + single
say str(single)

let arr = ["foo", "bar"]
let acc = ""
acc += arr[0]
acc += ":"
acc += arr[1]
say acc
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!(
                "csv: chrome.exe,code.exe,notepad.exe\n",
                "app: chrome.exe\n",
                "chrome.exe\n",
                "foo:bar\n",
            )
        );
    }
}

#[test]
fn test_e2e_string_relational_comparisons() {
    let code = r#"
// Lexicographical ordering
if "apple" < "banana"
    say "apple < banana ok"
end
if "banana" > "apple"
    say "banana > apple ok"
end
if "apple" <= "apple"
    say "apple <= apple ok"
end
if "apple" >= "apple"
    say "apple >= apple ok"
end
if "banana" < "apple"
    say "unreachable"
else
    say "banana < apple is false"
end

// Range comparison (like character class range check in regex)
let esc = "5"
if esc >= "1" and esc <= "9"
    say "5 in 1..9 ok"
end

let esc_low = "\x01"
if esc_low >= "1" and esc_low <= "9"
    say "unreachable"
else
    say "ctrl char out of 1..9 ok"
end

let esc_high = "z"
if esc_high >= "1" and esc_high <= "9"
    say "unreachable"
else
    say "z out of 1..9 ok"
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "apple < banana ok\n",
                "banana > apple ok\n",
                "apple <= apple ok\n",
                "apple >= apple ok\n",
                "banana < apple is false\n",
                "5 in 1..9 ok\n",
                "ctrl char out of 1..9 ok\n",
                "z out of 1..9 ok\n",
            )
        );
    }
}

#[test]
fn test_e2e_dynamic_equality_function_scope() {
    // `==`/`!=` on array elements inside function scope must compare by
    // value: untyped operands previously fell back to pointer compare
    // (issue #41). Mixed int/float-vs-string shapes previously crashed
    // outright instead of returning false.
    let code = r#"
function t1(a, b) -> int
    return a[0] == b[1]
end
function tne(a, b) -> int
    return a[0] != b[0]
end
function teq(a, b) -> int
    return a[0] == b[0]
end
function main()
    let s = ["h2"]
    let c = ["http/1.1", "h2"]
    say str(t1(s, c))
    say str(tne(s, c))
    say str(teq([1, 2], [1, 3]))
    say str(teq(["a", "b"], ["a", "c"]))
    say str(tne(["a"], ["b"]))
    let h = 123456789012345
    say str(h == "x")
    say str(h != "x")
    let f = 3.14
    say str(f == "x")
    say str("x" == 3.14)
    say str(5 == "x")
end
main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(output, "1\n1\n1\n1\n1\n0\n1\n0\n0\n0\n");
    }
}

#[test]
fn test_e2e_dynamic_equality_method_collision() {
    // Bare-name inference markers collide across functions
    // (`expected: string` in `assert_str_eq` mis-marks an int
    // `expected` elsewhere). Dynamic equality must verify both sides
    // at runtime instead of trusting one marking (issue #41).
    let code = r#"
import "std/test"
function main()
    let runner = runner_new()
    runner.assert_eq(5, 5, "equality check")
    runner.assert_eq(5, 6, "mismatch check")
    say "done"
end
main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert!(output.contains("done"), "Got: {}", output);
        assert!(output.contains("expected: 6"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_say_unknown_call_result() {
    // `say` of a call with statically-unknown return type must classify
    // at runtime instead of printing the raw pointer (issue #44).
    let code = r#"
function first_a(arr, name)
    for kid in arr
        if kid == name
            return kid
        end
    end
    return null
end
function main()
    let arr = ["x", "a", "b"]
    say first_a(arr, "a")
end
main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(output, "a\n");
    }
}

#[test]
fn test_e2e_string_store_outlives_ring() {
    // B1: stored strings must survive later concat volume. The runtime
    // string ring wraps ~950KB; pre-fix, array-held concat results were
    // silently overwritten (stale reads) and reading them segfaulted.
    // Push distinct content, burn past the wrap with different content,
    // then verify every element reads back intact.
    let code = r#"
function main()
    let arr = []
    arr.push("aa" + "bb")
    arr.push("cc" + "dd")
    arr.push("ee" + "ff")
    let d = ""
    let i = 0
    while i < 240000
        d = "wx" + "yz"
        i += 1
    end
    say arr[0]
    say arr[1]
    say arr[2]
    let h = ""
    h = h + arr[0]
    h = h + arr[1]
    h = h + arr[2]
    say h
end
main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(output, "aabb\nccdd\neeff\naabbccddeeff\n");
    }
}

#[test]
fn test_e2e_string_accum_across_wrap() {
    // B1: accumulating reads across the ring wrap must neither crash nor
    // corrupt. ~416-byte elements cross the ~950KB wrap within ~70
    // iterations; the final length and both ends are asserted exactly.
    let code = r#"
function big() -> string
    return "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
end
function main()
    let arr = []
    let i = 0
    while i < 80
        arr.push(big() + big())
        i += 1
    end
    let h = ""
    let k = 0
    while k < 80
        h = h + arr[k]
        k += 1
    end
    say len(h)
    say h[0..16]
    say h[len(h) - 16..len(h)]
end
main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3, "Got: {}", output);
        assert_eq!(lines[0], "33280", "Got: {}", output);
        assert_eq!(lines[1], "0123456789abcdef", "Got: {}", output);
        assert_eq!(lines[2], "0123456789abcdef", "Got: {}", output);
    }
}

#[test]
fn test_e2e_null_string_ops_no_crash() {
    // Regression: lower/upper/trim/substring on null segfaulted (0xC0000005).
    // Seen via App/vpn test_config run from the wrong directory, where
    // missing-file toml values flow null into lower(). Must return "".
    let code = r#"
let n = null
say "[" + lower(n) + "]"
say "[" + upper(n) + "]"
say "[" + trim(n) + "]"
say "[" + n.substring(1, 2) + "]"
say len(n)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(output, "[]\n[]\n[]\n[]\n0\n", "Got: {}", output);
    }
}

#[test]
fn test_e2e_dynamic_empty_string_classification() {
    // #92: a dynamic empty string (statically-unknown value whose first
    // byte is NUL) classified as int: say printed the pointer, == ""
    // was false, concat took the int path. Static empties always
    // worked; only the runtime classifier was wrong.
    let code = r#"
import "std/str" as str
let raw = str.split("1|50|newline|\n", chr(10))
let e = raw[0][13..13]
say len(e)
if e == ""
    say "eq-empty"
else
    say "ne-empty"
end
say e
say e + "x"
say "" + e
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(output, "0\neq-empty\n\nx\n\n", "Got: {}", output);
    }
}
