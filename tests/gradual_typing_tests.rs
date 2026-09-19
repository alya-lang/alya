use alya::codegen::analysis::type_checker::{parse_type_str, validate_types, Type};
use alya::lexer::Lexer;
use alya::parser::Parser;

fn parse_and_check(source: &str) -> Result<(), String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse()?;
    alya::parser::enums::resolve_enums(&mut ast);
    let _ = alya::parser::constants::resolve_and_validate_constants(&mut ast);
    alya::parser::generics::resolve_generics(&mut ast);
    validate_types(&ast)
}

#[test]
fn test_parse_type_strings() {
    assert_eq!(parse_type_str("int"), Type::Int);
    assert_eq!(parse_type_str("i32"), Type::I32);
    assert_eq!(parse_type_str("float"), Type::Float);
    assert_eq!(parse_type_str("f64"), Type::Float);
    assert_eq!(parse_type_str("string"), Type::String);
    assert_eq!(parse_type_str("bool"), Type::Bool);
    assert_eq!(parse_type_str("rune"), Type::Rune);
    assert_eq!(parse_type_str("void"), Type::Void);
    assert_eq!(parse_type_str("any"), Type::Any);
    assert_eq!(parse_type_str("int[]"), Type::Array(Box::new(Type::Int)));
    assert_eq!(
        parse_type_str("string?"),
        Type::Nullable(Box::new(Type::String))
    );
    assert_eq!(
        parse_type_str("(int, string)"),
        Type::Tuple(vec![Type::Int, Type::String])
    );
    assert_eq!(
        parse_type_str("[string: int]"),
        Type::Map(Box::new(Type::String), Box::new(Type::Int))
    );
    assert_eq!(parse_type_str("Point"), Type::Struct("Point".to_string()));
}

#[test]
fn test_type_assignability() {
    // Gradual typing: Any is assignable to/from anything
    assert!(Type::Int.is_assignable_to(&Type::Any));
    assert!(Type::Any.is_assignable_to(&Type::Int));
    assert!(Type::String.is_assignable_to(&Type::Any));
    assert!(Type::Any.is_assignable_to(&Type::String));

    // Primitives
    assert!(Type::Int.is_assignable_to(&Type::Int));
    assert!(Type::I32.is_assignable_to(&Type::Int));
    assert!(Type::Int.is_assignable_to(&Type::I32));
    assert!(Type::Float.is_assignable_to(&Type::Float));
    assert!(Type::F32.is_assignable_to(&Type::Float));
    assert!(Type::String.is_assignable_to(&Type::String));
    assert!(Type::Bool.is_assignable_to(&Type::Bool));

    // Nullable
    assert!(Type::Null.is_assignable_to(&Type::Nullable(Box::new(Type::Int))));
    assert!(Type::Int.is_assignable_to(&Type::Nullable(Box::new(Type::Int))));
    assert!(!Type::Null.is_assignable_to(&Type::Int));

    // Incompatibilities
    assert!(!Type::String.is_assignable_to(&Type::Int));
    assert!(!Type::Int.is_assignable_to(&Type::String));
    assert!(!Type::Float.is_assignable_to(&Type::String));
    assert!(!Type::String.is_assignable_to(&Type::Float));
    assert!(!Type::Array(Box::new(Type::Int)).is_assignable_to(&Type::Int));
}

#[test]
fn test_valid_typed_variable_declarations() {
    let code = r#"
let a: int = 42
let b: float = 3.14
let c: string = "hello"
let d: bool = true
let e: int[] = [1, 2, 3]
let f: string[] = ["x", "y"]
let g: int? = null
let h: (int, string) = (1, "one")
let m: [string: int] = {"count": 10}
"#;
    let res = parse_and_check(code);
    assert!(res.is_ok(), "Expected valid types, got: {:?}", res);
}

#[test]
fn test_type_mismatch_variable_declaration() {
    let code1 = r#"let x: int = "hello""#;
    let res1 = parse_and_check(code1);
    assert!(res1.is_err());
    assert!(res1.unwrap_err().contains("Type mismatch in 'let x'"));

    let code2 = r#"let x: float = "not a float""#;
    let res2 = parse_and_check(code2);
    assert!(res2.is_err());
    assert!(res2.unwrap_err().contains("Type mismatch in 'let x'"));

    let code3 = r#"let x: string = 12345"#;
    let res3 = parse_and_check(code3);
    assert!(res3.is_err());
    assert!(res3.unwrap_err().contains("Type mismatch in 'let x'"));

    let code4 = r#"let x: int = null"#;
    let res4 = parse_and_check(code4);
    assert!(res4.is_err());
    assert!(res4.unwrap_err().contains("Type mismatch in 'let x'"));
}

#[test]
fn test_type_mismatch_array_elements() {
    let code = r#"let nums: int[] = [1, 2, "three", 4]"#;
    let res = parse_and_check(code);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Type mismatch in array element"));
}

#[test]
fn test_type_mismatch_variable_reassignment() {
    let code = r#"
let x: int = 100
x = "changed to string"
"#;
    let res = parse_and_check(code);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .contains("Cannot assign 'string' to variable 'x' of type 'int'"));
}

#[test]
fn test_valid_function_signatures_and_returns() {
    let code = r#"
function add(a: int, b: int) -> int
    return a + b
end

function greet(name: string) -> string
    return "Hello, " + name
end

function do_nothing() -> void
    say "doing nothing"
end

let s = add(10, 20)
let msg = greet("Alya")
do_nothing()
"#;
    let res = parse_and_check(code);
    assert!(
        res.is_ok(),
        "Expected valid typed functions, got: {:?}",
        res
    );
}

#[test]
fn test_function_return_type_mismatch() {
    let code = r#"
function get_number() -> int
    return "this is a string, not int"
end
"#;
    let res = parse_and_check(code);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Return type mismatch"));
}

#[test]
fn test_void_function_returning_value() {
    let code = r#"
function proc() -> void
    return 42
end
"#;
    let res = parse_and_check(code);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .contains("Void function cannot return a value"));
}

#[test]
fn test_non_void_function_empty_return() {
    let code = r#"
function calc() -> int
    return
end
"#;
    let res = parse_and_check(code);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .contains("must return a value of type 'int'"));
}

#[test]
fn test_function_argument_type_mismatch() {
    let code = r#"
function square(n: int) -> int
    return n * n
end

let res = square("not a number")
"#;
    let res = parse_and_check(code);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .contains("Type mismatch for argument 1 of function 'square'"));
}

#[test]
fn test_struct_field_types_and_instantiation() {
    let valid_code = r#"
struct Point
    x: int
    y: int
end

let p = Point { x: 10, y: 20 }
"#;
    let res = parse_and_check(valid_code);
    assert!(
        res.is_ok(),
        "Expected valid struct instantiation, got: {:?}",
        res
    );

    let invalid_code = r#"
struct Point
    x: int
    y: int
end

let p = Point { x: "not int", y: 20 }
"#;
    let res2 = parse_and_check(invalid_code);
    assert!(res2.is_err());
    assert!(res2
        .unwrap_err()
        .contains("Type mismatch for field 'Point.x'"));
}

#[test]
fn test_struct_field_assignment_type_mismatch() {
    let code = r#"
struct Point
    x: int
    y: int
end

let p = Point { x: 1, y: 2 }
p.x = "string assigned to int field"
"#;
    let res = parse_and_check(code);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .contains("Type mismatch for field 'Point.x'"));
}

#[test]
fn test_gradual_typing_backward_compatibility() {
    // Completely unannotated code must pass without any errors
    let dynamic_code = r#"
let x = 10
let y = "hello"
let z = [1, "two", 3.0]

function dynamic_add(a, b)
    return a + b
end

let res = dynamic_add(x, 20)
let res2 = dynamic_add(y, " world")
"#;
    let res = parse_and_check(dynamic_code);
    assert!(
        res.is_ok(),
        "Expected dynamic code to pass cleanly, got: {:?}",
        res
    );
}

#[test]
fn test_gradual_typing_interoperability() {
    // Calling typed functions with untyped variables (inferred)
    let mixed_code = r#"
function typed_mul(a: int, b: int) -> int
    return a * b
end

let unannotated = 5
let result: int = typed_mul(unannotated, 10)
"#;
    let res = parse_and_check(mixed_code);
    assert!(
        res.is_ok(),
        "Expected mixed gradual code to pass, got: {:?}",
        res
    );
}
