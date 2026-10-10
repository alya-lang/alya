use super::{parse_code, Parser};
use crate::lexer::Lexer;
use crate::parser::CfgContext;
use std::collections::BTreeSet;

fn parse_with_cfg(code: &str, ctx: CfgContext) -> Result<crate::ast::Program, String> {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.set_cfg_context(ctx);
    parser.parse()
}

fn ctx_for(features: &[&str]) -> CfgContext {
    CfgContext::for_target(
        "linux",
        "x64",
        true,
        &features.iter().map(|s| s.to_string()).collect(),
    )
}

const TWO_FNS: &str = "@cfg(feature = \"simd\")\nfunction fast()\n    return 1\nend\n\nfunction slow()\n    return 2\nend\n";

#[test]
fn test_cfg_feature_enabled_keeps_branch() {
    let prog = parse_with_cfg(TWO_FNS, ctx_for(&["simd"])).unwrap();
    assert_eq!(prog.statements.len(), 2);
}

#[test]
fn test_cfg_feature_disabled_drops_branch() {
    let prog = parse_with_cfg(TWO_FNS, ctx_for(&[])).unwrap();
    assert_eq!(prog.statements.len(), 1);
}

#[test]
fn test_cfg_unknown_feature_is_false_not_error() {
    // Unknown names are false at compile time (multi-manifest safety);
    // `alya lint` flags the typo against the manifest.
    let prog = parse_with_cfg(TWO_FNS, ctx_for(&["other"])).unwrap();
    assert_eq!(prog.statements.len(), 1);
}

#[test]
fn test_cfg_debug_flag() {
    let code = "@cfg(debug = true)\nfunction d()\n    return 1\nend\n";
    let on = CfgContext::for_target("linux", "x64", true, &BTreeSet::new());
    assert_eq!(parse_with_cfg(code, on).unwrap().statements.len(), 1);
    let off = CfgContext::for_target("linux", "x64", false, &BTreeSet::new());
    assert_eq!(parse_with_cfg(code, off).unwrap().statements.len(), 0);
    // Malformed debug value is a hard error.
    let bad = "@cfg(debug = yes)\nfunction d()\n    return 1\nend\n";
    let err = parse_with_cfg(bad, ctx_for(&[])).unwrap_err();
    assert!(err.contains("Invalid @cfg debug value"), "got: {}", err);
}

#[test]
fn test_cfg_unknown_key_is_error() {
    let code = "@cfg(osss = \"linux\")\nfunction f()\n    return 1\nend\n";
    let err = parse_with_cfg(code, ctx_for(&[])).unwrap_err();
    assert!(err.contains("Unknown @cfg key 'osss'"), "got: {}", err);
}

#[test]
fn test_cfg_target_not_host() {
    // Explicit target context: host OS never leaks in.
    let code = "@cfg(os = \"macos\")\nfunction m()\n    return 1\nend\n";
    let mac = CfgContext::for_target("macos", "arm64", true, &BTreeSet::new());
    assert_eq!(parse_with_cfg(code, mac).unwrap().statements.len(), 1);
    let win = CfgContext::for_target("windows", "x64", true, &BTreeSet::new());
    assert_eq!(parse_with_cfg(code, win).unwrap().statements.len(), 0);
}

#[test]
fn test_cfg_not_negation() {
    let code = "@cfg(not(feature = \"simd\"))\nfunction f()\n    return 1\nend\n";
    assert_eq!(
        parse_with_cfg(code, ctx_for(&[])).unwrap().statements.len(),
        1
    );
    assert_eq!(
        parse_with_cfg(code, ctx_for(&["simd"]))
            .unwrap()
            .statements
            .len(),
        0
    );
}

#[test]
fn test_cfg_stacked_lines_conjoin() {
    let code = "@cfg(os = \"linux\")\n@cfg(feature = \"simd\")\nfunction f()\n    return 1\nend\n";
    assert_eq!(
        parse_with_cfg(code, ctx_for(&["simd"]))
            .unwrap()
            .statements
            .len(),
        1
    );
    assert_eq!(
        parse_with_cfg(code, ctx_for(&[])).unwrap().statements.len(),
        0
    );
}

#[test]
fn test_cfg_bare_attribute_keeps_statement() {
    let code = "@cfg\nfunction f()\n    return 1\nend\n";
    assert_eq!(
        parse_with_cfg(code, ctx_for(&[])).unwrap().statements.len(),
        1
    );
}

#[test]
fn test_cfg_dropped_branch_still_parses() {
    // Disabled branches are parsed (syntax errors surface) but dropped.
    let code = "@cfg(feature = \"off\")\nfunction broken( \nend\n";
    assert!(parse_with_cfg(code, ctx_for(&[])).is_err());
    // ... while well-formed code parses fine, proving the error above
    // comes from parsing the disabled branch, not from dropping.
    let _ = parse_code("function ok()\n    return 1\nend\n").unwrap();
}

// alya-lang/alya#156: `any(...)` disjunction over conditions.
const ANY_FNS: &str =
    "@cfg(any(feature = \"a\", feature = \"b\"))\nfunction f()\n    return 1\nend\n";

#[test]
fn test_cfg_any_keeps_when_one_branch_holds() {
    assert_eq!(
        parse_with_cfg(ANY_FNS, ctx_for(&["b"]))
            .unwrap()
            .statements
            .len(),
        1
    );
    assert_eq!(
        parse_with_cfg(ANY_FNS, ctx_for(&["a", "b"]))
            .unwrap()
            .statements
            .len(),
        1
    );
}

#[test]
fn test_cfg_any_drops_when_no_branch_holds() {
    assert_eq!(
        parse_with_cfg(ANY_FNS, ctx_for(&[]))
            .unwrap()
            .statements
            .len(),
        0
    );
    // Unknown feature names are false inside branches too.
    assert_eq!(
        parse_with_cfg(ANY_FNS, ctx_for(&["other"]))
            .unwrap()
            .statements
            .len(),
        0
    );
}

#[test]
fn test_cfg_any_single_branch_and_mixed_kinds() {
    let code = "@cfg(any(os = \"linux\"))\nfunction f()\n    return 1\nend\n";
    assert_eq!(
        parse_with_cfg(code, ctx_for(&[])).unwrap().statements.len(),
        1
    );
    let code = "@cfg(any(debug = false, feature = \"a\"))\nfunction f()\n    return 1\nend\n";
    assert_eq!(
        parse_with_cfg(code, ctx_for(&["a"]))
            .unwrap()
            .statements
            .len(),
        1
    );
    assert_eq!(
        parse_with_cfg(code, ctx_for(&[])).unwrap().statements.len(),
        0
    );
}

#[test]
fn test_cfg_any_nesting_and_negation() {
    // Nested `any(...)` composes; commas inside nesting never split.
    let code = "@cfg(any(feature = \"x\", any(feature = \"a\", feature = \"b\")))\nfunction f()\n    return 1\nend\n";
    assert_eq!(
        parse_with_cfg(code, ctx_for(&["b"]))
            .unwrap()
            .statements
            .len(),
        1
    );
    assert_eq!(
        parse_with_cfg(code, ctx_for(&[])).unwrap().statements.len(),
        0
    );
    // `not(any(...))` gates "none of these features".
    let code =
        "@cfg(not(any(feature = \"a\", feature = \"b\")))\nfunction f()\n    return 1\nend\n";
    assert_eq!(
        parse_with_cfg(code, ctx_for(&[])).unwrap().statements.len(),
        1
    );
    assert_eq!(
        parse_with_cfg(code, ctx_for(&["a"]))
            .unwrap()
            .statements
            .len(),
        0
    );
    // De Morgan fallback: `any(not(a), not(b))` keeps unless both on.
    let code =
        "@cfg(any(not(feature = \"a\"), not(feature = \"b\")))\nfunction f()\n    return 1\nend\n";
    assert_eq!(
        parse_with_cfg(code, ctx_for(&["a"]))
            .unwrap()
            .statements
            .len(),
        1
    );
    assert_eq!(
        parse_with_cfg(code, ctx_for(&["a", "b"]))
            .unwrap()
            .statements
            .len(),
        0
    );
}

#[test]
fn test_cfg_any_empty_is_false_and_bad_key_errors() {
    // Empty disjunction holds nothing.
    let code = "@cfg(any())\nfunction f()\n    return 1\nend\n";
    assert_eq!(
        parse_with_cfg(code, ctx_for(&[])).unwrap().statements.len(),
        0
    );
    // Unknown keys stay hard errors inside branches.
    let code = "@cfg(any(feature = \"a\", osss = \"linux\"))\nfunction f()\n    return 1\nend\n";
    let err = parse_with_cfg(code, ctx_for(&[])).unwrap_err();
    assert!(err.contains("Unknown @cfg key 'osss'"), "got: {}", err);
}
