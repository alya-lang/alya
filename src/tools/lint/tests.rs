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
fn test_lint_unused_import_from_symbols_in_fstrings() {
    // Regression: `from` symbols used only inside f-string interpolations
    // count as used; escaped `{{...}}` does not; genuinely unused symbols
    // are still flagged.
    let tmp = std::env::temp_dir().join(format!("alya_lint_fstr_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let file = tmp.join("user.alya");
    let source = "from \"std/math\" import PI, floor, ceil, random, nosuchfn, ghostfn\nsay f\"Direct Pi: {PI}\"\nsay f\"Direct floor: {floor(7.9)}\"\nsay f\"Direct ceil: {ceil(7.1)}\"\nsay f\"Direct random: {random()}\"\nsay f\"escaped {{ghostfn}} braces\"\n";
    std::fs::write(&file, source).unwrap();
    let diags = lint_source(source, &file).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    let names: Vec<_> = import_diags.iter().map(|d| d.message.clone()).collect();
    assert_eq!(
        import_diags.len(),
        2,
        "only truly unused symbols: {:?}",
        names
    );
    assert!(names.iter().any(|m| m.contains("nosuchfn")));
    assert!(names.iter().any(|m| m.contains("ghostfn")));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_lint_unused_import_plain_string_braces_count() {
    // #155: only f-strings interpolate, so only f-string holes count as
    // usage; braces in a plain string reference nothing.
    let tmp = std::env::temp_dir().join(format!("alya_lint_pstr_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let file = tmp.join("user.alya");
    let source = "from \"std/math\" import floor\nsay f\"use {floor} notation\"\n";
    std::fs::write(&file, source).unwrap();
    let diags = lint_source(source, &file).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert_eq!(import_diags.len(), 0);
    let plain = "from \"std/math\" import floor\nsay \"use {floor} notation\"\n";
    std::fs::write(&file, plain).unwrap();
    let diags = lint_source(plain, &file).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert_eq!(import_diags.len(), 1);
    let _ = std::fs::remove_dir_all(&tmp);
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
fn test_lint_unused_import_unresolvable_target_silent() {
    // alya-lang/alya#111: a lone copy (missing siblings) hides the
    // target's exports from per-file analysis. The import must stay
    // silent — flagging it deletes live code (`Regex` in return-type
    // positions, `compile_regex`/`escape_pattern` calls).
    let tmp = std::env::temp_dir().join(format!("alya_lint_lone_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let file = tmp.join("lib.alya");
    let source = r#"import "./types.alya"
import "./utils.alya"
import "./compiler.alya"

pub function compile(pattern: string) -> Regex
    return compile_regex(pattern)
end

pub function Regex.is_match(self: Regex, text: string) -> int
    return is_match(self, text)
end
"#;
    std::fs::write(&file, source).unwrap();
    let diags = lint_source(source, &file).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert!(
        import_diags.is_empty(),
        "unresolvable targets must stay silent, got: {:?}",
        import_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_lint_unused_import_resolvable_dead_still_fires() {
    // The silence above must not swallow genuine dead imports: with the
    // sibling present and none of its exports referenced, the warning
    // (and its whole-line fix) still fires.
    let tmp = std::env::temp_dir().join(format!("alya_lint_dead_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(
        tmp.join("dead.alya"),
        "pub struct Unused\nend\npub function never_called() -> int\n    return 0\nend\n",
    )
    .unwrap();
    let file = tmp.join("user.alya");
    let source = "import \"./dead.alya\"\n\nfunction main()\n    say 1\nend\n";
    std::fs::write(&file, source).unwrap();
    let diags = lint_source(source, &file).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert_eq!(import_diags.len(), 1);
    assert!(import_diags[0].fix.is_some());
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
fn test_lint_unused_import_facade_reexport_suppressed() {
    // Regression test: `src/lib.alya`-style facades re-export submodule
    // symbols to downstream `alias::symbol` consumers (and link them in).
    // An import whose symbols are unused *locally* must NOT be flagged when
    // the file declares `pub` items — flagging it led `--fix` to strip the
    // import and break downstream builds with `undefined reference` link
    // errors (semver `bumper`, term `cursor`/`live` incidents).
    let tmp = std::env::temp_dir().join(format!("alya_lint_facade_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(
        tmp.join("bumper.alya"),
        "pub function bump_major(v: int) -> int\n    return v + 1\nend\n",
    )
    .unwrap();

    let lib_file = tmp.join("lib.alya");
    let source = r#"
import "./bumper.alya"

pub function facade_version() -> int
    return 1
end
"#;
    let diags = lint_source(source, &lib_file).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert!(
        import_diags.is_empty(),
        "facade re-export import must stay silent, got: {:?}",
        import_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_lint_unused_import_leaf_still_flagged_with_fix() {
    // Same shape as the facade test, but the importing file declares no
    // `pub`/`extern` items, so nobody can consume symbols through it: the
    // import is genuinely dead and keeps its warning + `--fix`.
    let tmp = std::env::temp_dir().join(format!("alya_lint_leaf_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(
        tmp.join("bumper.alya"),
        "pub function bump_major(v: int) -> int\n    return v + 1\nend\n",
    )
    .unwrap();

    let caller_file = tmp.join("caller.alya");
    let source = r#"
import "./bumper.alya"

function main()
    say "hi"
end
"#;
    let diags = lint_source(source, &caller_file).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert_eq!(import_diags.len(), 1);
    assert_eq!(
        import_diags[0].message,
        "imported module 'bumper' is never used"
    );
    assert!(
        import_diags[0].fix.is_some(),
        "leaf-file unused import must keep its auto-fix"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_lint_unused_import_facade_empty_module_still_flagged() {
    // A facade importing a module that exports nothing has no re-export to
    // protect: removal cannot break downstream symbol resolution, so the
    // warning (with fix) is preserved.
    let tmp = std::env::temp_dir().join(format!("alya_lint_facade_empty_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(
        tmp.join("helper.alya"),
        "function helper() -> int\n    return 1\nend\n",
    )
    .unwrap();

    let lib_file = tmp.join("lib.alya");
    let source = r#"
import "./helper.alya"

pub function facade_version() -> int
    return 2
end
"#;
    let diags = lint_source(source, &lib_file).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert_eq!(import_diags.len(), 1);
    assert!(
        import_diags[0].fix.is_some(),
        "empty-export import removal is safe and must keep its auto-fix"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_lint_unused_import_aliased_unused_in_facade_flagged() {
    // Regression (alya-lang/alya#75): an `as`-aliased import that is never
    // referenced is dead even when the importing file declares `pub` items.
    // Aliases namespace the module for local use only — neither
    // `facade::symbol` nor `facade::alias::symbol` resolves downstream
    // (verified against the compiler), so no re-export protection applies
    // and the verdict must not depend on whether the target resolves
    // (i.e. CI linting before `alya install` vs. local runs with a warm
    // `.alya/packages` cache).
    let tmp = std::env::temp_dir().join(format!("alya_lint_aliased_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(
        tmp.join("helper.alya"),
        "pub function helper_add(a: int, b: int) -> int\n    return a + b\nend\n",
    )
    .unwrap();

    let lib_file = tmp.join("lib.alya");
    let source = r#"
import "./helper.alya" as helper

pub function facade_version() -> int
    return 2
end
"#;
    let diags = lint_source(source, &lib_file).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert_eq!(import_diags.len(), 1);
    assert_eq!(
        import_diags[0].message,
        "imported module 'helper' is never used"
    );
    assert!(
        import_diags[0].fix.is_some(),
        "dead aliased import removal is safe and must keep its auto-fix"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_lint_unused_import_aliased_used_stays_silent() {
    // Guard against false positives: an `as`-aliased import referenced via
    // its namespace must stay silent, including in `pub`-bearing files.
    let tmp = std::env::temp_dir().join(format!("alya_lint_aliased_used_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    std::fs::write(
        tmp.join("helper.alya"),
        "pub function helper_add(a: int, b: int) -> int\n    return a + b\nend\n",
    )
    .unwrap();

    let lib_file = tmp.join("lib.alya");
    let source = r#"
import "./helper.alya" as helper

pub function facade_version() -> int
    return helper::helper_add(1, 2)
end
"#;
    let diags = lint_source(source, &lib_file).unwrap();
    let import_diags: Vec<_> = diags.iter().filter(|d| d.rule == "unused-import").collect();
    assert!(
        import_diags.is_empty(),
        "used aliased import must stay silent, got: {:?}",
        import_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let _ = std::fs::remove_dir_all(&tmp);
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
fn test_lint_idiomatic_style_elif_heterogeneous_no_when() {
    // Chains with heterogeneous predicates have no statement-`when`
    // equivalent (argumentless `when` is expression-only): no suggestion.
    let source = r#"
function parse_tok(tok)
    let tlen = len(tok)
    if tlen >= 2 and substring(tok, 0, 2) == "0x"
        say "hex"
    elif is_date_or_time(tok) == 1
        say "date"
    elif has_char(tok, ".") == 1
        say "float"
    else
        say "int"
    end
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let style_diags: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "idiomatic-style" && d.message.contains("if/elif"))
        .collect();
    assert!(
        style_diags.is_empty(),
        "heterogeneous chain must not suggest 'when', got: {:?}",
        style_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_lint_idiomatic_style_elif_ne_chain_no_when() {
    // `!=` arms would need branch inversion: no suggestion.
    let source = r#"
function check(v)
    if v != 1
        say "a"
    elif v != 2
        say "b"
    elif v != 3
        say "c"
    else
        say "d"
    end
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let style_diags: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "idiomatic-style" && d.message.contains("if/elif"))
        .collect();
    assert!(style_diags.is_empty(), "`!=` chains must stay silent");
}

#[test]
fn test_lint_idiomatic_style_elif_dotted_subject_fires() {
    // Same dotted subject across arms: convertible, names the subject.
    let source = r#"
function check(line)
    if line.indent == 1
        say "a"
    elif line.indent == 2
        say "b"
    elif line.indent == 3
        say "c"
    else
        say "d"
    end
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let style_diags: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "idiomatic-style" && d.message.contains("if/elif"))
        .collect();
    assert_eq!(style_diags.len(), 1);
    assert!(
        style_diags[0].message.contains("line.indent"),
        "suggestion should name the subject, got: {}",
        style_diags[0].message
    );
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
    let check_res = run_lint_cli(
        Some(temp_dir.to_str().unwrap()),
        false,
        true,
        LintFormat::Text,
        None,
        &[],
        false,
    );
    assert!(check_res.is_err(), "CI check gate should fail on warnings");

    // Fix mode should apply the fix
    let fix_res = run_lint_cli(
        Some(temp_dir.to_str().unwrap()),
        true,
        false,
        LintFormat::Text,
        None,
        &[],
        false,
    );
    assert!(fix_res.is_ok(), "Fix mode should succeed");

    let fixed_content = fs::read_to_string(&file_path).unwrap();
    assert!(fixed_content.contains("_unused_test_var"));

    // Now check mode should succeed
    let check_res_after = run_lint_cli(
        Some(temp_dir.to_str().unwrap()),
        false,
        true,
        LintFormat::Text,
        None,
        &[],
        false,
    );
    assert!(check_res_after.is_ok(), "Check should pass once fixed");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_lint_cli_sarif_output() {
    let temp_dir =
        std::env::temp_dir().join(format!("alya_test_lint_sarif_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    fs::write(
        temp_dir.join("main.alya"),
        "function main()\n    let unused_sarif_var = 99\n    say \"hello\"\nend\n",
    )
    .unwrap();
    let sarif_path = temp_dir.join("alya-lint.sarif");

    // SARIF artifact is written even though --check fails: CI uploads first.
    let res = run_lint_cli(
        Some(temp_dir.to_str().unwrap()),
        false,
        true,
        LintFormat::Sarif,
        Some(sarif_path.to_str().unwrap()),
        &[],
        false,
    );
    assert!(res.is_err(), "Check gate must still fail on warnings");

    let payload = fs::read_to_string(&sarif_path).unwrap();
    let log: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(log["version"], "2.1.0");
    let results = log["runs"][0]["results"].as_array().unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r["ruleId"] == "unused-var"));
    for r in results {
        assert!(
            r["locations"][0]["physicalLocation"]["region"]["startLine"]
                .as_u64()
                .unwrap()
                >= 1
        );
    }

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

#[test]
fn test_lint_cfg_unknown_feature() {
    let tmp = std::env::temp_dir().join(format!("alya_lint_cfgfeat_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("src")).unwrap();
    std::fs::write(
        tmp.join("alya.toml"),
        "[package]\nname = \"cfgapp\"\nversion = \"0.1.0\"\nentry = \"src/main.alya\"\n\n[features]\ndefault = []\nsimd = []\n",
    )
    .unwrap();
    let file = tmp.join("src").join("main.alya");
    let source = "@cfg(feature = \"simd\")\nfunction f()\n    return 1\nend\n@cfg(feature = \"simdd\")\nfunction g()\n    return 2\nend\n@cfg(not(feature = \"nope\"))\nfunction h()\n    return 3\nend\n";
    std::fs::write(&file, source).unwrap();
    let diags = lint_source(source, &file).unwrap();
    let cfg_diags: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "cfg-unknown-feature")
        .collect();
    assert_eq!(
        cfg_diags.len(),
        2,
        "typos flagged: {:?}",
        cfg_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert!(cfg_diags.iter().any(|d| d.message.contains("'simdd'")));
    assert!(cfg_diags.iter().any(|d| d.message.contains("'nope'")));

    // Outside packages (no manifest) the rule stays silent.
    let lone = std::env::temp_dir().join(format!("alya_lint_cfglone_{}.alya", std::process::id()));
    let lone_src = "@cfg(feature = \"anything\")\nfunction f()\n    return 1\nend\n";
    std::fs::write(&lone, lone_src).unwrap();
    let lone_diags = lint_source(lone_src, &lone).unwrap();
    assert!(lone_diags.iter().all(|d| d.rule != "cfg-unknown-feature"));
    let _ = std::fs::remove_file(&lone);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_lint_uses_package_default_features() {
    // Gated code is analyzed as ACTIVE under package defaults: a parameter
    // used only inside `@cfg(feature)` must not warn in the default view,
    // but must warn once defaults are disabled.
    let tmp = std::env::temp_dir().join(format!("alya_lint_featdef_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("src")).unwrap();
    std::fs::write(
        tmp.join("alya.toml"),
        "[package]\nname = \"featapp\"\nversion = \"0.1.0\"\nentry = \"src/main.alya\"\n\n[features]\ndefault = [\"accel\"]\naccel = []\n",
    )
    .unwrap();
    let file = tmp.join("src").join("main.alya");
    let source = "function make(use_accel = true)\n    @cfg(feature = \"accel\")\n    if use_accel != false\n        return 1\n    end\n    return 0\nend\n";
    std::fs::write(&file, source).unwrap();

    let default_diags = lint_source(source, &file).unwrap();
    assert!(
        default_diags.iter().all(|d| d.rule != "unused-param"),
        "default view keeps gated use: {:?}",
        default_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let slim_diags = lint_source_with_cli_features(source, &file, &[], true).unwrap();
    assert!(
        slim_diags.iter().any(|d| d.rule == "unused-param"),
        "slim view drops gated use: {:?}",
        slim_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    // Explicit opt-in re-enables the gated use even with defaults off.
    let opt_in =
        lint_source_with_cli_features(source, &file, &["accel".to_string()], true).unwrap();
    assert!(opt_in.iter().all(|d| d.rule != "unused-param"));

    // Unknown CLI features are rejected like `alya build` does.
    assert!(lint_source_with_cli_features(source, &file, &["nope".to_string()], false).is_err());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_lint_cli_no_default_features_gate() {
    // End-to-end: `run_lint_cli` with `--no-default-features` fails `--check`
    // on gated-only uses that pass in the default view.
    let tmp = std::env::temp_dir().join(format!("alya_lint_featcli_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(
        tmp.join("alya.toml"),
        "[package]\nname = \"featcli\"\nversion = \"0.1.0\"\nentry = \"main.alya\"\n\n[features]\ndefault = [\"accel\"]\naccel = []\n",
    )
    .unwrap();
    std::fs::write(
        tmp.join("main.alya"),
        "function make(use_accel = true)\n    @cfg(feature = \"accel\")\n    if use_accel != false\n        return 1\n    end\n    return 0\nend\n",
    )
    .unwrap();

    let default_res = run_lint_cli(
        Some(tmp.to_str().unwrap()),
        false,
        true,
        LintFormat::Text,
        None,
        &[],
        false,
    );
    assert!(default_res.is_ok(), "default view should pass --check");

    let slim_res = run_lint_cli(
        Some(tmp.to_str().unwrap()),
        false,
        true,
        LintFormat::Text,
        None,
        &[],
        true,
    );
    assert!(slim_res.is_err(), "slim view should fail --check");

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_lint_dynamic_is_float_on_map_read() {
    let source = r#"
function probe(m)
    if m["k"] is float
        say "f"
    end
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "dynamic-is-float")
        .collect();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].message.contains("best-effort"));
}

#[test]
fn test_lint_dynamic_is_float_quiet_on_literal() {
    let source = r#"
function probe()
    if 0.5 is float
        say "f"
    end
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    assert!(diags.iter().all(|d| d.rule != "dynamic-is-float"));
}

#[test]
fn test_lint_dynamic_is_float_quiet_on_annotated_call() {
    let source = r#"
function get_temp() -> float
    return 0.5
end
function probe()
    if get_temp() is float
        say "f"
    end
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    assert!(diags.iter().all(|d| d.rule != "dynamic-is-float"));
}

#[test]
fn test_lint_dynamic_is_float_fires_on_untyped_call() {
    let source = r#"
function get_val()
    return 0.5
end
function probe()
    if get_val() is float
        say "f"
    end
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "dynamic-is-float")
        .collect();
    assert_eq!(hits.len(), 1);
}

#[test]
fn test_lint_boolean_literals_return_in_predicate() {
    let source = r#"
function is_valid(x) -> int
    if x == 0
        return 0
    end
    return 1
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "boolean-literals")
        .collect();
    // 2 literal returns + 1 annotation = 3 findings, all informational.
    assert_eq!(
        hits.len(),
        3,
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    for d in &hits {
        assert_eq!(d.severity, crate::tools::lint::types::LintSeverity::Info);
        assert!(d.fix.is_some());
    }
    let lits: Vec<_> = hits
        .iter()
        .filter(|d| d.message.contains("boolean return value"))
        .collect();
    assert_eq!(lits.len(), 2);
    assert!(lits.iter().all(|d| {
        let r = &d.fix.as_ref().unwrap().replacement;
        r == "true" || r == "false"
    }));
    let ann: Vec<_> = hits
        .iter()
        .filter(|d| d.message.contains("-> bool"))
        .collect();
    assert_eq!(ann.len(), 1);
    assert_eq!(ann[0].fix.as_ref().unwrap().replacement, "bool");
}

#[test]
fn test_lint_boolean_literals_non_predicate_silent() {
    let source = r#"
function count_matches(s, sub) -> int
    if len(s) == 0
        return 0
    end
    return 1
end
function mode_all() -> int
    return 0
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "boolean-literals")
        .collect();
    assert!(
        hits.is_empty(),
        "expected silence, got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_lint_boolean_literals_predicate_comparison() {
    let source = r#"
function check(x)
    if is_valid(x) == 1
        say "yes"
    end
    if len(x) == 0
        say "empty"
    end
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "boolean-literals")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert_eq!(hits[0].fix.as_ref().unwrap().replacement, "true");
}

#[test]
fn test_lint_boolean_literals_tuple_and_loop_idiom_silent() {
    let source = r#"
function get_move(dx) -> int
    if dx != 1
        return 0, -1
    end
    return 1, 0
end
function spin()
    while 1 == 1
        break
    end
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "boolean-literals")
        .collect();
    assert!(
        hits.is_empty(),
        "expected silence, got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_lint_method_self_recursion_fires() {
    // No same-file free `is_positive`: guaranteed infinite loop -> Warning.
    let source = r#"
struct Box
    val: int
end
function Box.is_positive(self: Box) -> int
    return is_positive(self)
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "method-self-recursion")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert!(hits[0].message.contains("never terminates"));
    assert_eq!(
        hits[0].severity,
        crate::tools::lint::types::LintSeverity::Warning
    );
}

#[test]
fn test_lint_method_self_recursion_delegation_is_info() {
    // Same-file free `is_positive` exists: the call delegates via the
    // compiler guard, so this is informational fragility, not a hang.
    let source = r#"
struct Box
    val: int
end
function is_positive(b: Box) -> int
    if b.val > 0
        return 1
    end
    return 0
end
function Box.is_positive(self: Box) -> int
    return is_positive(self)
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "method-self-recursion")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert!(hits[0].message.contains("delegates"));
    assert_eq!(
        hits[0].severity,
        crate::tools::lint::types::LintSeverity::Info
    );
}

#[test]
fn test_lint_method_self_recursion_spans_call_site() {
    // alya-lang/alya#111: the diagnostic must span the call site, not
    // the callee's definition — with several same-bare delegations,
    // each instance lands on its own call.
    let source = r#"pub function is_match(reg, text: string) -> int
    return 1
end
pub function find(reg, text: string) -> int
    return 2
end
struct Regex
    pattern: string
end
pub function Regex.is_match(self: Regex, text: string) -> int
    return is_match(self, text)
end
pub function Regex.find(self: Regex, text: string) -> int
    if text == ""
        return 0
    end
    return find(self, text)
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "method-self-recursion")
        .collect();
    assert_eq!(
        hits.len(),
        2,
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    // Call sites are on lines 11 and 17 (no leading newline here); the
    // callee definitions are on lines 1 and 4.
    assert_eq!(hits[0].line, 11, "first hit must span its call");
    assert_eq!(hits[1].line, 17, "second hit must span its call");
    for h in &hits {
        assert!(h.col > 1, "span must carry the real column, got {:?}", h);
        assert!(h.end_col > h.col, "span must cover the name, got {:?}", h);
    }
}

#[test]
fn test_lint_method_self_recursion_genuine_recursion_silent() {
    let source = r#"
function count_down(n: int) -> int
    if n <= 0
        return 0
    end
    return count_down(n - 1) + 1
end
struct Box
    val: int
end
function Box.step(self: Box, n: int) -> int
    if n <= 0
        return self.val
    end
    return self.step(n - 1)
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "method-self-recursion")
        .collect();
    assert!(
        hits.is_empty(),
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_lint_duplicate_map_key_fires_and_fixes() {
    let source = r#"
function main()
    let m = { "a": 1, "a": 2 }
    say m["a"]
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "duplicate-map-key")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert!(hits[0].fix.is_some());
}

#[test]
fn test_lint_duplicate_map_key_ternary_silent() {
    let source = r#"
function pick(c)
    let m = { "k": c ? "a" : "b" }
    return m["k"]
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "duplicate-map-key")
        .collect();
    assert!(
        hits.is_empty(),
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_lint_nul_byte_in_string_fires() {
    // alya-lang/alya#129: embedded NUL truncates at runtime with no
    // error, so the literal must be loud.
    let source = "function main()\n    let s = \"a\\0b\"\n    say s\nend\n";
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "nul-byte-in-string")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_lint_nul_byte_in_string_clean() {
    // Plain strings and NUL runes (legitimate terminator checks) stay
    // quiet; only string payloads carrying NUL fire.
    let source = "function main()\n    let s = \"ab\"\n    let r = '\\0'\n    say s\n    say r == '\\0'\nend\n";
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "nul-byte-in-string")
        .collect();
    assert!(
        hits.is_empty(),
        "got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_lint_null_equality_fires_and_fixes() {
    let source = r#"
function check(x)
    if x == null
        say "nil"
    end
    if x != null
        say "set"
    end
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags.iter().filter(|d| d.rule == "null-equality").collect();
    assert_eq!(
        hits.len(),
        2,
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let reps: Vec<_> = hits
        .iter()
        .map(|d| d.fix.as_ref().unwrap().replacement.clone())
        .collect();
    assert!(reps.contains(&"is null".to_string()));
    assert!(reps.contains(&"is not null".to_string()));
}

#[test]
fn test_lint_compound_assign_fires_and_fixes() {
    let source = r#"
function bump()
    let i = 0
    i = i + 1
    s.count = s.count + 2
    return i
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "compound-assign")
        .collect();
    assert_eq!(
        hits.len(),
        2,
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let reps: Vec<_> = hits
        .iter()
        .map(|d| d.fix.as_ref().unwrap().replacement.clone())
        .collect();
    assert!(reps.contains(&"i +=".to_string()));
    assert!(reps.contains(&"s.count +=".to_string()));
}

#[test]
fn test_lint_compound_assign_call_arg_silent() {
    let source = r#"
function draw(w, h)
    box(w, h + 1)
    let t = total(count, count + 1)
    arr[i] = arr[i] + 1
    say t
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "compound-assign")
        .collect();
    assert!(
        hits.is_empty(),
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_lint_compound_assign_mixed_operators_silent() {
    // alya-lang/alya#110: `x OP= REST` regroups the RHS around OP, so the
    // rule must stay silent unless OP is the loosest operator and no
    // same-level follower regroups unsoundly. Every shape below broke
    // semantics when rewritten (`res = res * 16 + (c - 48)` became
    // `res *= 16 + (c - 48)` and stuck the accumulator at 0).
    let source = r#"
function scan(c)
    let res = 0
    res = res * 16 + (c - 48)
    res = res * 8 + (c - 48)
    res = res * 2 + 1
    res = res - c - 1
    res = res * c / 2
    res = res / c * 2
    res = res % c + 1
    res = res + c == 1
    return res
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "compound-assign")
        .collect();
    assert!(
        hits.is_empty(),
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn test_lint_compound_assign_sound_shapes_fire() {
    // Same-level followers that regroup exactly (`+` then `+`/`-`,
    // `*` then `*`), parenthesized tails, unary operands, and tighter
    // trailing operators keep their suggestions.
    let source = r#"
function bump(a, b, c)
    a = a + 1
    a = a + b - c
    a = a * b * c
    a = a + b * c
    a = a - b * c
    a = a + (b - c)
    a = a * -b
    return a
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "compound-assign")
        .collect();
    assert_eq!(
        hits.len(),
        7,
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    let reps: Vec<_> = hits
        .iter()
        .map(|d| d.fix.as_ref().unwrap().replacement.clone())
        .collect();
    assert_eq!(reps.iter().filter(|r| *r == "a +=").count(), 4);
    assert_eq!(reps.iter().filter(|r| *r == "a *=").count(), 2);
    assert_eq!(reps.iter().filter(|r| *r == "a -=").count(), 1);
}

#[test]
fn test_lint_float_equality_fires() {
    let source = r#"
function check(f: float) -> int
    if f == 0.0
        return 1
    end
    return 0
end
"#;
    let diags = lint_source(source, Path::new("test.alya")).unwrap();
    let hits: Vec<_> = diags
        .iter()
        .filter(|d| d.rule == "float-equality")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert!(hits[0].fix.is_none());
}
