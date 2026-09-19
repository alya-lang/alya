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
    assert_eq!(unused_diags[0].message, "variable 'unused_val' is declared but never read");
    assert_eq!(unused_diags[0].fix.as_ref().unwrap().replacement, "_unused_val");
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
    assert!(unused_diags.is_empty(), "Underscore variables should not be flagged");
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
    assert!(param_diags[0].message.contains("parameter 'z' is defined in function 'add' but never used"));
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
    assert_eq!(import_diags[0].message, "imported module 'os' is never used");
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
    assert_eq!(dead_diags[0].message, "unreachable statement following 'return'");
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
    let style_diags: Vec<_> = diags.iter().filter(|d| d.rule == "idiomatic-style").collect();
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
