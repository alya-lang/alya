//! Optimization-level parity: every supported `opt-level` must execute
//! programs identically. The peephole passes only remove provably dead or
//! redundant instructions, so outputs across O0-O3 must be byte-identical
//! (and correct). This is the miscompilation net for `codegen::peephole`.
mod common;
use common::*;

const CASES: &[(&str, &str)] = &[
    (
        "fib-recursion",
        "function fib(n: int) -> int\n    if n <= 1\n        return n\n    end\n    return fib(n - 1) + fib(n - 2)\nend\n\nsay fib(22)\n",
    ),
    (
        "loop-array",
        "let total = 0\nlet xs = []\nfor i in 0..200\n    push(xs, i)\n    total += xs[i] * 2\nend\nsay total\nsay len(xs)\n",
    ),
    (
        "strings",
        "let s = \"\"\nfor i in 0..50\n    s += f\"{i},\"\nend\nsay len(s)\nsay s\n",
    ),
    (
        "floats",
        "let acc = 0.0\nfor i in 0..1000\n    acc += i * 0.5 - 0.25\nend\nsay acc\n",
    ),
    (
        "structs",
        "struct Pt\n    x: int\n    y: int\nend\n\nlet p = Pt { x: 3, y: 4 }\nsay p.x + p.y\nsay p.x * p.y\n",
    ),
];

#[test]
fn test_opt_levels_execute_identically() {
    for (name, source) in CASES {
        let mut outputs = Vec::new();
        for opt in 0..=3u8 {
            match run_alya_code_full_with_opt(source, opt) {
                Some((0, out)) => outputs.push(out),
                Some((code, out)) => {
                    panic!("case '{}' at O{} exited {}:\n{}", name, opt, code, out)
                }
                None => {
                    println!("SKIP case '{}' at O{}: no toolchain", name, opt);
                    continue;
                }
            }
        }
        if outputs.is_empty() {
            continue;
        }
        for (i, out) in outputs.iter().enumerate().skip(1) {
            assert_eq!(
                *out, outputs[0],
                "case '{}': O{} output differs from O0",
                name, i
            );
        }
        println!("  [OPT PARITY] {:<14} identical across O0-O3", name);
    }
}

#[test]
fn test_opt_parity_with_features() {
    // cfg-gated branches under optimization: enabled and fallback paths
    // must both execute identically across levels.
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/syntax/attributes.alya");
    let source = std::fs::read_to_string(&path).expect("attributes.alya readable");
    for features in [vec!["spec_proof".to_string()], Vec::<String>::new()] {
        let mut outputs = Vec::new();
        for opt in 0..=3u8 {
            let with_cfg = run_alya_code_full_with_features_and_opt(&source, &features, opt);
            match with_cfg {
                Some((0, out)) => outputs.push(out),
                Some((code, out)) => panic!(
                    "attributes.alya features={:?} O{} exited {}:\n{}",
                    features, opt, code, out
                ),
                None => {
                    println!("SKIP attributes O{}: no toolchain", opt);
                    continue;
                }
            }
        }
        if outputs.is_empty() {
            continue;
        }
        for (i, out) in outputs.iter().enumerate().skip(1) {
            assert_eq!(
                *out, outputs[0],
                "attributes.alya features={:?}: O{} differs from O0",
                features, i
            );
        }
    }
}
