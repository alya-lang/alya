mod common;
use common::*;

#[test]
fn test_e2e_functions() {
    let code = r#"
function multiply(x, y)
    return x * y
end

say multiply(7, 6)
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "42\n");
    }
}

#[test]
fn test_e2e_module_import() {
    let pid = std::process::id();
    let mod_filename = format!("temp_imported_helper_{}.alya", pid);
    let mod_content = r#"
function compute_bonus(salary)
    return salary * 2
end
"#;
    fs::write(&mod_filename, mod_content).expect("Failed to write temporary module file");

    let main_code = format!(
        r#"
import "{}"
let base = 1000
let total = compute_bonus(base)
say total
"#,
        mod_filename
    );

    let res = run_alya_code_full(&main_code);
    let _ = fs::remove_file(&mod_filename);

    if let Some((code, output)) = res {
        assert_eq!(code, 0);
        assert_eq!(output, "2000\n");
    }
}

#[test]
fn test_e2e_from_selective_import() {
    let pid = std::process::id();
    let mod_filename = format!("temp_imported_calc_{}.alya", pid);
    let mod_content = r#"
function calc_add(a, b)
    return a + b
end

function calc_mul(a, b)
    return a * b
end

function calc_hidden(a)
    return a * 10
end
"#;
    fs::write(&mod_filename, mod_content).expect("Failed to write temporary module file");

    let main_code = format!(
        r#"
from "{}" import calc_add, calc_mul as multiply
say calc_add(15, 25)
say multiply(6, 7)
"#,
        mod_filename
    );

    let res = run_alya_code_full(&main_code);
    let _ = fs::remove_file(&mod_filename);

    if let Some((code, output)) = res {
        assert_eq!(code, 0);
        assert_eq!(output, "40\n42\n");
    }
}

#[test]
fn test_e2e_from_selective_import_missing_symbol_error() {
    let pid = std::process::id();
    let mod_filename = format!("temp_imported_calc_err_{}.alya", pid);
    let mod_content = r#"
function calc_add(a, b)
    return a + b
end
"#;
    fs::write(&mod_filename, mod_content).expect("Failed to write temporary module file");

    let main_code = format!(
        r#"
from "{}" import non_existent_fn
say non_existent_fn()
"#,
        mod_filename
    );

    let mut lexer = alya::lexer::Lexer::new(&main_code);
    let tokens = lexer.tokenize().expect("Lexer error");
    let mut parser = alya::parser::Parser::new(tokens);
    let mut ast = parser.parse().expect("Parser error");
    let res = alya::parser::resolve_imports(&mut ast, std::path::Path::new("."));
    let _ = fs::remove_file(&mod_filename);

    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .contains("does not export symbol 'non_existent_fn'"));
}

#[test]
fn test_e2e_for_each_loop() {
    let code = r#"
let nums = [10, 20, 30]
for n in nums
    say n
end

for x in [1, 2, 3]
    say x * 2
end

let fruits = ["apple", "banana", "cherry"]
for f in fruits
    say "fruit: {f}"
end

for x in [1, 2, 3, 4, 5]
    if x == 2
        continue
    end
    if x == 4
        break
    end
    say x
end

let empty = []
for item in empty
    say "should not print"
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "10\n",
                "20\n",
                "30\n",
                "2\n",
                "4\n",
                "6\n",
                "fruit: apple\n",
                "fruit: banana\n",
                "fruit: cherry\n",
                "1\n",
                "3\n",
            )
        );
    }
}

#[test]
fn test_e2e_parameter_type_propagation() {
    let code = r#"
function append_tag(tags, val)
    tags.push(val)
end

function process(arr, dict, label)
    append_tag(arr, label)
    dict["tag"] = label
end

let items = ["init"]
let data = map()
process(items, data, "test_run")
say items[0]
say items[1]
say data["tag"]
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, concat!("init\n", "test_run\n", "test_run\n",));
    }
}

#[test]
fn test_e2e_default_parameters() {
    let code = r#"
function greet(name, greeting = "Hello", punctuation = "!")
    say "{greeting}, {name}{punctuation}"
end

greet("World")
greet("Alice", "Hi")
greet("Bob", "Good morning", "?")

function power(base, exp = 2)
    let result = 1
    for i in 1..exp
        result *= base
    end
    return result
end

say power(3)
say power(2, 4)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "Hello, World!\n",
                "Hi, Alice!\n",
                "Good morning, Bob?\n",
                "9\n",
                "16\n",
            )
        );
    }
}

#[test]
fn test_e2e_multiple_returns_and_tuples() {
    let code = r#"
function min_max(a, b)
    if a < b
        return a, b
    else
        return b, a
    end
end

let lo, hi = min_max(50, 20)
say lo
say hi

let (x, y) = min_max(10, 99)
say x
say y

let t = (100, 200, 300)
say len(t)
say t[0]
say t[1]
say t[2]

let a = 1
let b = 2
a, b = b, a
say a
say b

(a, b) = (b, a)
say a
say b

function user_info()
    return "Alice", 30
end

let name, age = user_info()
say name
say age
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "20\n", "50\n", "10\n", "99\n", "3\n", "100\n", "200\n", "300\n", "2\n", "1\n",
                "1\n", "2\n", "Alice\n", "30\n",
            )
        );
    }
}

#[test]
fn test_e2e_destructured_string_propagation() {
    let code = r#"
function extract_pair()
    let x = "GET"
    let y = "/api/status"
    return x, y
end

function format_header(verb, path)
    return "[" + verb + "] " + path
end

function handle()
    let m, p = extract_pair()
    if p == "/api/status"
        let res = format_header(m, p)
        say res
    else
        say "failed match"
    end
end

handle()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "[GET] /api/status\n");
    }
}

#[test]
fn test_e2e_defer_statement() {
    let code = r#"
# 1. Basic LIFO execution on function exit
function basic_defer()
    defer say "defer 1 (declared first, runs last)"
    defer say "defer 2 (declared middle, runs middle)"
    defer say "defer 3 (declared last, runs first)"
    say "function body"
end

say "--- basic ---"
basic_defer()

# 2. Defer preserving return value
function defer_with_return()
    defer say "defer executed before return"
    say "inside compute"
    return 100 + 23
end

say "--- with return ---"
let result = defer_with_return()
say result

# 3. Conditional defer with early exit
function conditional_defer(flag)
    defer say "defer always active"
    if flag
        say "taking early exit branch"
        return "early"
    end
    defer say "defer only in normal branch"
    say "taking normal branch"
    return "normal"
end

say "--- conditional early ---"
let r1 = conditional_defer(true)
say r1

say "--- conditional normal ---"
let r2 = conditional_defer(false)
say r2
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "--- basic ---\n",
                "function body\n",
                "defer 3 (declared last, runs first)\n",
                "defer 2 (declared middle, runs middle)\n",
                "defer 1 (declared first, runs last)\n",
                "--- with return ---\n",
                "inside compute\n",
                "defer executed before return\n",
                "123\n",
                "--- conditional early ---\n",
                "taking early exit branch\n",
                "defer always active\n",
                "early\n",
                "--- conditional normal ---\n",
                "taking normal branch\n",
                "defer only in normal branch\n",
                "defer always active\n",
                "normal\n",
            )
        );
    }
}

#[test]
fn test_e2e_lambda_basic() {
    let code = r#"
let double = fn(x) => x * 2
say double(21)

let add = fn(a, b) => a + b
say add(15, 27)

let greet = fn(name) => "hello " + name
say greet("alya")
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, concat!("42\n", "42\n", "hello alya\n",));
    }
}

#[test]
fn test_e2e_lambda_higher_order() {
    let code = r#"
function apply(f, val)
    return f(val)
end

function apply_twice(f, val)
    return f(f(val))
end

say apply(fn(x) => x * 3, 7)
say apply_twice(fn(n) => n + 10, 5)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, concat!("21\n", "25\n",));
    }
}

#[test]
fn test_e2e_named_function_as_value() {
    let code = r#"
function triple(x)
    return x * 3
end

let f = triple
say f(4)

function run_func(f, val)
    return f(val)
end

say run_func(triple, 10)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, concat!("12\n", "30\n",));
    }
}

#[test]
fn test_e2e_direct_lambda_call() {
    let code = r#"
say (fn(x) => x + 100)(50)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "150\n");
    }
}

#[test]
fn test_e2e_block_lambda_multiline() {
    let code = r#"
# 1. Assigned multi-line block lambda
let compute = fn(a, b)
    let temp = a * 2
    let res = temp + b
    return res
end
say compute(10, 5)

# 2. Passed as argument to higher-order function
function exec_fn(f, arg)
    return f(arg)
end

let out = exec_fn(fn(x)
    let y = x * 3
    return y + 10
end, 100)
say out

# 3. Anonymous function with function(...) keyword
let adder = function(x, y)
    let sum = x + y
    return sum
end
say adder(20, 30)

# 4. Multi-line lambda with string return
let greet = fn(name)
    let msg = "Hello " + name + "!"
    return msg
end
say greet("World")
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, concat!("25\n", "310\n", "50\n", "Hello World!\n",));
    }
}
