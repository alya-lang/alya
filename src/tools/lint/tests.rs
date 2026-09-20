use super::*;
use std::fs;
use std::path::Path;

#[test]
fn test_lint_unused_var() {
    let source = r#"
function compute()
    let unused_val = 100
    let used_val = 50
    say used_val
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let unused_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-var").collect();
    assert_eq!(unused_diags.len(), 1);
    assert_eq!(
        unused_diags[0].message,
        "variable 'unused_val' is declared but never read"
    );
    assert_eq!(
        unused_diags[0].fix.as_ref().unwrap().replacement,
        "_unused_val"
    );
}

#[test]
fn test_lint_unused_var_scoped_location_across_multiple_functions() {
    let source = r#"
function first_fn()
    let res = 10
    say res
end

function second_fn()
    let res = 20
    say 42
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let unused_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-var").collect();
    assert_eq!(unused_diags.len(), 1);
    assert_eq!(
        unused_diags[0].message,
        "variable 'res' is declared but never read"
    );
    // Crucial check: line must point to second_fn (line 8), NOT first_fn (line 3)
    assert_eq!(
        unused_diags[0].line, 8,
        "Must point to second_fn, not first_fn"
    );
}

#[test]
fn test_lint_ignored_underscore_var() {
    let source = r#"
function compute()
    let _ignored = 100
    say 42
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let unused_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-var").collect();
    assert!(
        unused_diags.is_empty(),
        "Underscore variables should not be flagged"
    );
}

#[test]
fn test_lint_unused_param() {
    let source = r#"
function add(x, y, z)
    return x + y
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let param_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-param").collect();
    assert_eq!(param_diags.len(), 1);
    assert!(param_diags[0]
        .message
        .contains("parameter 'z' is defined in function 'add' but never used"));
    assert_eq!(param_diags[0].fix.as_ref().unwrap().replacement, "_z");
}

#[test]
fn test_lint_unused_import() {
    let source = r#"
import "std/math"
import "std/os"

function main()
    let val = math::sqrt(16.0)
    say val
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert_eq!(import_diags.len(), 1);
    assert_eq!(
        import_diags[0].message,
        "imported module 'os' is never used"
    );
}

#[test]
fn test_lint_unused_import_keyword_assert() {
    let source = r#"
import "std/test"

function main()
    assert(1 == 1, "passing")
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert_eq!(import_diags.len(), 0);
}

#[test]
fn test_lint_unused_import_relative_file_and_struct_methods() {
    let tmp = std::env::temp_dir().join(format!("alya_lint_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mod_file = tmp.join("math.alya");
    std::fs::write(
        &mod_file,
        "pub struct MyMath\nend\npub function MyMath.compute(x) -> int\n    return x * 2\nend\n",
    )
    .unwrap();

    let caller_file = tmp.join("caller.alya");
    let source = r#"
import "./math.alya"

function main()
    let x = MyMath.compute(10)
    say x
end
"#;
    let diags = lint_source(source, &caller_file).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert_eq!(import_diags.len(), 0);

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_lint_unused_import_exported_symbols_and_keywords() {
    let source = r#"
import "std/hash"
import "std/test"
import "std/os"

function main()
    let h = murmur3("hello")
    test.assert_eq(h, h, "deterministic")
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert_eq!(import_diags.len(), 1);
    assert_eq!(
        import_diags[0].message,
        "imported module 'os' is never used"
    );
}

#[test]
fn test_lint_dead_code_following_return() {
    let source = r#"
function run()
    return 42
    say "this is dead code"
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let dead_diags: Vec<_> = diags.iter().filter(|d| d.rule == "dead-code").collect();
    assert_eq!(dead_diags.len(), 1);
    assert_eq!(
        dead_diags[0].message,
        "unreachable statement following 'return'"
    );
}

#[test]
fn test_lint_idiomatic_style_elif_chain() {
    let source = r#"
function check(x)
    if x == 1
        say "one"
    elif x == 2
        say "two"
    elif x == 3
        say "three"
    else
        say "other"
    end
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let style_diags: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "idiomatic-style")
        .collect();
    assert_eq!(style_diags.len(), 1);
    assert!(style_diags[0].message.contains("long 'if/elif' chain"));
    assert!(style_diags[0].help.as_ref().unwrap().contains("when"));
}

#[test]
fn test_lint_apply_fixes() {
    let source = r#"
function compute(x, unused_param)
    let my_unused = 123
    return x * 2
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let (fixed_source, count) = apply_fixes_to_source(source, &diags);
    assert_eq!(count, 2);
    assert!(fixed_source.contains("_unused_param"));
    assert!(fixed_source.contains("let _my_unused = 123"));
}

#[test]
fn test_lint_cli_check_gate() {
    let temp_dir = std::env::temp_dir().join(format!("alya_test_lint_cli_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("main.alya");
    fs::write(
        &file_path,
        "function main()\n    let unused_test_var = 99\n    say \"hello\"\nend\n",
    )
    .unwrap();

    // Check mode should fail because there is an unused var
    let check_res = run_lint_cli(Some(temp_dir.to_str().unwrap()), false, true);
    assert!(check_res.is_err(), "CI check gate should fail on warnings");

    // Fix mode should apply the fix
    let fix_res = run_lint_cli(Some(temp_dir.to_str().unwrap()), true, false);
    assert!(fix_res.is_ok(), "Fix mode should succeed");

    let fixed_content = fs::read_to_string(&file_path).unwrap();
    assert!(fixed_content.contains("_unused_test_var"));

    // Now check mode should succeed
    let check_res_after = run_lint_cli(Some(temp_dir.to_str().unwrap()), false, true);
    assert!(check_res_after.is_ok(), "Check should pass once fixed");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_lint_redundant_return_variable() {
    let source = r#"
function get_shuffled(rng, arr)
    let shuf = sample_shuffled(rng, arr)
    return shuf
end

function compute()
    let res = 10 + 20
    return res
end

function not_redundant(arr)
    let x = arr[0]
    say x
    return x
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let redundant_diags: Vec<_> = diags
        .iter()
        .filter(|d| {
            d.rule == "idiomatic-style" && d.message.contains("redundant variable assignment")
        })
        .collect();

    assert_eq!(redundant_diags.len(), 2);
    assert_eq!(redundant_diags[0].line, 3);
    assert!(redundant_diags[0].message.contains("'shuf'"));
    assert_eq!(redundant_diags[1].line, 8);
    assert!(redundant_diags[1].message.contains("'res'"));

    // Verify autofix works
    let (fixed, count) = apply_fixes_to_source(source, &diags);
    assert_eq!(count, 2);
    assert!(fixed.contains("return sample_shuffled(rng, arr)"));
    assert!(fixed.contains("return 10 + 20"));
}

#[test]
fn test_lint_suppression_directives() {
    let source = r#"
function compute()
    # alya-lint: disable-next-line unused-var
    let intentional_unused = 123
    let real_unused = 456
    say 1
end

# alya-lint: disable unused-var
function another()
    let ignored1 = 1
    let ignored2 = 2
end
# alya-lint: enable unused-var

function third()
    let trailing_ignored = 999 # alya-ignore
    let not_ignored = 888
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let unused_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-var").collect();

    let names: Vec<&str> = unused_diags
        .iter()
        .map(|d| {
            let start = d.message.find('\'').unwrap() + 1;
            let end = d.message[start..].find('\'').unwrap() + start;
            &d.message[start..end]
        })
        .collect();

    assert!(names.contains(&"real_unused"));
    assert!(names.contains(&"not_ignored"));
    assert!(!names.contains(&"intentional_unused"));
    assert!(!names.contains(&"ignored1"));
    assert!(!names.contains(&"ignored2"));
    assert!(!names.contains(&"trailing_ignored"));
}

#[test]
fn test_lint_suspicious_bugs_rules() {
    let source = r#"
function test_bugs(x)
    if x == x
        say "always true"
    end
    if true
        say "constant true"
    end
    while false
        say "dead loop"
    end
    x + 100
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let self_cmp = diags.iter().any(|d| d.rule == "self-comparison");
    let const_cond = diags.iter().any(|d| d.rule == "constant-condition");
    let useless_expr = diags.iter().any(|d| d.rule == "useless-expression");

    assert!(self_cmp, "Should detect self-comparison 'x == x'");
    assert!(const_cond, "Should detect constant-condition 'if true'");
    assert!(useless_expr, "Should detect useless-expression 'x + 100'");
}

#[test]
fn test_lint_naming_conventions() {
    let source = r#"
function badNamedFunction()
    say 1
end

struct bad_struct_name
    x: int
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let naming_diags: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "naming-convention")
        .collect();

    assert_eq!(naming_diags.len(), 2);
    assert!(naming_diags[0].message.contains("badNamedFunction"));
    assert!(naming_diags[1].message.contains("bad_struct_name"));
}

#[test]
fn test_lint_config_disabled_rules_and_exclude() {
    let temp_dir = std::env::temp_dir().join(format!("alya_test_lint_cfg_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("vendor")).unwrap();

    let toml_content = r#"
[package]
name = "test_pkg"
version = "0.1.0"

[lint]
disabled_rules = ["naming-convention"]
exclude = ["vendor"]
"#;
    fs::write(temp_dir.join("alya.toml"), toml_content).unwrap();

    // Vendor file should be excluded
    fs::write(
        temp_dir.join("vendor/lib.alya"),
        "function badName()\n    let unused = 1\nend\n",
    )
    .unwrap();

    // Main file
    fs::write(
        temp_dir.join("main.alya"),
        "function badName()\n    say 42\nend\n",
    )
    .unwrap();

    let diags = lint_source(
        "function badName()\n    say 42\nend\n",
        &temp_dir.join("main.alya"),
    )
    .unwrap();
    let naming_diags: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "naming-convention")
        .collect();
    assert_eq!(
        naming_diags.len(),
        0,
        "naming-convention should be disabled via alya.toml"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
