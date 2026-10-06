//! Round-trip acceptance gate (hybrid B first slice).
//!
//! Dump → read → dump byte-identical over the full differential corpus
//! (`spec/syntax` + `stdlib`), plus `Program` equality. This is gate 1 of
//! the `CODEGEN_FEASIBILITY.md` bridge design; execution parity (gate 2)
//! arrives with pipeline wiring.

use alya::ast::{Expr, Program, Stmt};
use alya::driver::{ast_sexpr, sexpr_reader};
use alya::lexer::Lexer;
use alya::parser::Parser;
use std::fs;
use std::path::PathBuf;

fn corpus_dirs() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spec = manifest.join("spec").join("syntax");
    let stdlib = manifest.join("stdlib");
    // Fallback for workspaces that nest the crate one level deeper.
    let alt_spec = manifest.parent().unwrap().join("spec").join("syntax");
    let alt_stdlib = manifest.parent().unwrap().join("stdlib");
    vec![
        if spec.exists() { spec } else { alt_spec },
        if stdlib.exists() { stdlib } else { alt_stdlib },
    ]
}

/// Truncate every string at the first NUL, mirroring the dumper (`qs`)
/// and `diff_parse.py`: NUL cannot survive the dump, so AST equality only
/// holds modulo this documented lossy rule.
fn trunc(s: &str) -> String {
    s.split('\0').next().unwrap_or("").to_string()
}

fn norm_expr(e: &Expr) -> Expr {
    match e {
        Expr::Number(n) => Expr::Number(*n),
        Expr::Float(f) => Expr::Float(*f),
        Expr::String(s) => Expr::String(trunc(s)),
        Expr::Identifier(n) => Expr::Identifier(trunc(n)),
        Expr::Null => Expr::Null,
        Expr::Binary { left, op, right } => Expr::Binary {
            left: Box::new(norm_expr(left)),
            op: *op,
            right: Box::new(norm_expr(right)),
        },
        Expr::Unary { op, expr } => Expr::Unary {
            op: *op,
            expr: Box::new(norm_expr(expr)),
        },
        Expr::Call { name, args } => Expr::Call {
            name: trunc(name),
            args: args.iter().map(norm_expr).collect(),
        },
        Expr::InterpolatedString(ps) => {
            Expr::InterpolatedString(ps.iter().map(norm_expr).collect())
        }
        Expr::Array(es) => Expr::Array(es.iter().map(norm_expr).collect()),
        Expr::Index { array, index } => Expr::Index {
            array: Box::new(norm_expr(array)),
            index: Box::new(norm_expr(index)),
        },
        Expr::FieldAccess { object, field } => Expr::FieldAccess {
            object: Box::new(norm_expr(object)),
            field: trunc(field),
        },
        Expr::StructInit { name, fields } => Expr::StructInit {
            name: trunc(name),
            fields: fields
                .iter()
                .map(|(n, v)| (trunc(n), norm_expr(v)))
                .collect(),
        },
        Expr::Map(kvs) => Expr::Map(
            kvs.iter()
                .map(|(k, v)| (norm_expr(k), norm_expr(v)))
                .collect(),
        ),
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => Expr::Ternary {
            condition: Box::new(norm_expr(condition)),
            then_branch: Box::new(norm_expr(then_branch)),
            else_branch: Box::new(norm_expr(else_branch)),
        },
        Expr::NullCoalesce { value, default } => Expr::NullCoalesce {
            value: Box::new(norm_expr(value)),
            default: Box::new(norm_expr(default)),
        },
        Expr::OptionalFieldAccess { object, field } => Expr::OptionalFieldAccess {
            object: Box::new(norm_expr(object)),
            field: trunc(field),
        },
        Expr::OptionalIndex { array, index } => Expr::OptionalIndex {
            array: Box::new(norm_expr(array)),
            index: Box::new(norm_expr(index)),
        },
        Expr::OptionalCall { callee, args } => Expr::OptionalCall {
            callee: trunc(callee),
            args: args.iter().map(norm_expr).collect(),
        },
        Expr::ForceUnwrap(x) => Expr::ForceUnwrap(Box::new(norm_expr(x))),
        Expr::TypeCheck {
            expr,
            target,
            negated,
        } => Expr::TypeCheck {
            expr: Box::new(norm_expr(expr)),
            target: trunc(target),
            negated: *negated,
        },
        Expr::Cast { expr, target } => Expr::Cast {
            expr: Box::new(norm_expr(expr)),
            target: trunc(target),
        },
    }
}

fn norm_stmt(s: &Stmt) -> Stmt {
    match s {
        Stmt::Import {
            path,
            alias,
            symbols,
        } => Stmt::Import {
            path: trunc(path),
            alias: alias.as_ref().map(|a| trunc(a)),
            symbols: symbols.as_ref().map(|v| {
                v.iter()
                    .map(|im| alya::ast::ImportSymbol {
                        name: trunc(&im.name),
                        alias: im.alias.as_ref().map(|a| trunc(a)),
                    })
                    .collect()
            }),
        },
        Stmt::Say(e) => Stmt::Say(norm_expr(e)),
        Stmt::Let {
            name,
            type_ann,
            value,
        } => Stmt::Let {
            name: trunc(name),
            type_ann: type_ann.as_ref().map(|a| trunc(a)),
            value: norm_expr(value),
        },
        Stmt::Const { name, value } => Stmt::Const {
            name: trunc(name),
            value: norm_expr(value),
        },
        Stmt::Assign { name, value } => Stmt::Assign {
            name: trunc(name),
            value: norm_expr(value),
        },
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => Stmt::IndexAssign {
            array: norm_expr(array),
            index: norm_expr(index),
            value: norm_expr(value),
        },
        Stmt::FieldAssign {
            object,
            field,
            value,
        } => Stmt::FieldAssign {
            object: norm_expr(object),
            field: trunc(field),
            value: norm_expr(value),
        },
        // Non-string-carrying shapes compare as-is via re-dump identity;
        // normalize only the common shapes above, else clone.
        Stmt::ExternBlock {
            abi,
            lib,
            functions,
        } => Stmt::ExternBlock {
            abi: trunc(abi),
            lib: lib.as_ref().map(|a| trunc(a)),
            functions: functions
                .iter()
                .map(|f| alya::ast::ExternFnDecl {
                    name: trunc(&f.name),
                    params: f
                        .params
                        .iter()
                        .map(|p| alya::ast::ExternParam {
                            name: trunc(&p.name),
                            param_type: p.param_type.as_ref().map(|a| trunc(a)),
                        })
                        .collect(),
                    return_type: f.return_type.as_ref().map(|a| trunc(a)),
                })
                .collect(),
        },
        Stmt::StructDef {
            name,
            fields,
            field_types,
            defaults,
            attributes,
        } => Stmt::StructDef {
            name: trunc(name),
            fields: fields.iter().map(|f| trunc(f)).collect(),
            field_types: field_types
                .iter()
                .map(|t| t.as_ref().map(|a| trunc(a)))
                .collect(),
            defaults: defaults.iter().map(|d| d.as_ref().map(norm_expr)).collect(),
            attributes: norm_attrs(attributes),
        },
        Stmt::EnumDef { name, variants } => Stmt::EnumDef {
            name: trunc(name),
            variants: variants
                .iter()
                .map(|(n, v)| (trunc(n), v.as_ref().map(norm_expr)))
                .collect(),
        },
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => Stmt::If {
            condition: norm_expr(condition),
            then_block: norm_block(then_block),
            else_block: else_block.as_ref().map(|b| norm_block(b)),
        },
        Stmt::While { condition, body } => Stmt::While {
            condition: norm_expr(condition),
            body: norm_block(body),
        },
        Stmt::Repeat { body } => Stmt::Repeat {
            body: norm_block(body),
        },
        Stmt::For {
            var,
            start,
            end,
            inclusive,
            body,
        } => Stmt::For {
            var: trunc(var),
            start: norm_expr(start),
            end: norm_expr(end),
            inclusive: *inclusive,
            body: norm_block(body),
        },
        Stmt::ForEach {
            var,
            value_var,
            iterable,
            body,
        } => Stmt::ForEach {
            var: trunc(var),
            value_var: value_var.as_ref().map(|a| trunc(a)),
            iterable: norm_expr(iterable),
            body: norm_block(body),
        },
        Stmt::Function {
            name,
            params,
            param_types,
            return_type,
            defaults,
            body,
            type_params,
            attributes,
        } => Stmt::Function {
            name: trunc(name),
            params: params.iter().map(|p| trunc(p)).collect(),
            param_types: param_types
                .iter()
                .map(|t| t.as_ref().map(|a| trunc(a)))
                .collect(),
            return_type: return_type.as_ref().map(|a| trunc(a)),
            defaults: defaults.iter().map(|d| d.as_ref().map(norm_expr)).collect(),
            body: norm_block(body),
            type_params: type_params.iter().map(|t| trunc(t)).collect(),
            attributes: norm_attrs(attributes),
        },
        Stmt::Return(e) => Stmt::Return(e.as_ref().map(norm_expr)),
        Stmt::Break => Stmt::Break,
        Stmt::Continue => Stmt::Continue,
        Stmt::Expr(e) => Stmt::Expr(norm_expr(e)),
        Stmt::Throw(e) => Stmt::Throw(e.as_ref().map(norm_expr)),
        Stmt::TryCatch {
            try_block,
            catch_var,
            catch_block,
            finally_block,
        } => Stmt::TryCatch {
            try_block: norm_block(try_block),
            catch_var: catch_var.as_ref().map(|a| trunc(a)),
            catch_block: norm_block(catch_block),
            finally_block: finally_block.as_ref().map(|b| norm_block(b)),
        },
        Stmt::Defer(inner) => Stmt::Defer(Box::new(norm_stmt(inner))),
        Stmt::Pub(inner) => Stmt::Pub(Box::new(norm_stmt(inner))),
        Stmt::InterfaceDef {
            name,
            methods,
            embedded,
        } => Stmt::InterfaceDef {
            name: trunc(name),
            methods: methods
                .iter()
                .map(|m| alya::ast::InterfaceMethod {
                    name: trunc(&m.name),
                    params: m.params.iter().map(|p| trunc(p)).collect(),
                    param_types: m
                        .param_types
                        .iter()
                        .map(|t| t.as_ref().map(|a| trunc(a)))
                        .collect(),
                    return_type: m.return_type.as_ref().map(|a| trunc(a)),
                })
                .collect(),
            embedded: embedded.iter().map(|e| trunc(e)).collect(),
        },
    }
}

fn norm_block(b: &[Stmt]) -> Vec<Stmt> {
    b.iter().map(norm_stmt).collect()
}

fn norm_attrs(a: &[alya::ast::Attribute]) -> Vec<alya::ast::Attribute> {
    a.iter()
        .map(|at| alya::ast::Attribute {
            name: trunc(&at.name),
            args: at
                .args
                .iter()
                .map(|(k, v)| (k.as_ref().map(|s| trunc(s)), trunc(v)))
                .collect(),
        })
        .collect()
}

fn norm_prog(p: &Program) -> Program {
    Program {
        statements: p.statements.iter().map(norm_stmt).collect(),
    }
}
fn collect_corpus() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in corpus_dirs() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for e in entries.flatten() {
                if e.path().extension().is_some_and(|x| x == "alya") {
                    files.push(e.path());
                }
            }
        }
    }
    files.sort();
    files
}

#[test]
fn test_sexpr_roundtrip_corpus() {
    let files = collect_corpus();
    assert!(
        files.len() >= 40,
        "corpus too small ({} files), check corpus_dirs()",
        files.len()
    );
    let mut passed = 0;
    let mut skipped = 0;
    let mut failed: Vec<String> = Vec::new();
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let source = fs::read_to_string(path).unwrap();
        let tokens = match Lexer::new(&source).tokenize() {
            Ok(t) => t,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let prog = match Parser::new(tokens).parse_raw() {
            Ok(p) => p,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let dump = ast_sexpr::program_to_sexpr(&prog);
        let back = match sexpr_reader::parse_program(&dump) {
            Ok(p) => p,
            Err(e) => {
                failed.push(format!("{name}: reader: {e}"));
                continue;
            }
        };
        if norm_prog(&back) != norm_prog(&prog) {
            failed.push(format!("{name}: AST mismatch"));
            continue;
        }
        let dump2 = ast_sexpr::program_to_sexpr(&back);
        if dump2 != dump {
            failed.push(format!("{name}: re-dump mismatch"));
            continue;
        }
        passed += 1;
    }
    for f in &failed {
        println!("ROUNDTRIP FAIL: {f}");
    }
    println!(
        "roundtrip: passed={passed} skipped={skipped} failed={}",
        failed.len()
    );
    assert!(
        failed.is_empty(),
        "{} corpus files failed round-trip",
        failed.len()
    );
    assert!(
        passed >= 40,
        "only {passed} files round-tripped, want >= 40"
    );
}

#[test]
fn test_sexpr_reader_rejects_diagnostic_lines() {
    // `0|0|error|...` never reaches the reader in the pipeline.
    assert!(sexpr_reader::parse_program("0|0|error|3:7").is_err());
    assert!(sexpr_reader::parse_program("")
        .unwrap()
        .statements
        .is_empty());
}
