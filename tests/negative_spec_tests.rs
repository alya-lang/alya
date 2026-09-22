use alya::lexer::Lexer;
use alya::parser::Parser;
use std::fs;
use std::path::PathBuf;

fn get_negative_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let embedded = manifest_dir.join("spec").join("negative");
    if embedded.exists() {
        embedded
    } else {
        manifest_dir.parent().unwrap().join("spec").join("negative")
    }
}

fn parse_expectation(source: &str) -> (String, String) {
    let first = source.lines().next().unwrap_or("");
    let rest = first.strip_prefix("# EXPECT-FAIL:").unwrap_or("").trim();
    let mut parts = rest.splitn(2, ':');
    (
        parts.next().unwrap_or("").trim().to_string(),
        parts.next().unwrap_or("").trim().to_string(),
    )
}

fn run_check_stage(ast: &alya::ast::Program, base_dir: &std::path::Path) -> Result<(), String> {
    let mut resolved = ast.clone();
    alya::parser::resolve_imports(&mut resolved, base_dir)?;
    alya::parser::enums::resolve_enums(&mut resolved);
    alya::parser::constants::resolve_and_validate_constants(&mut resolved)?;
    alya::parser::generics::resolve_generics(&mut resolved);
    alya::codegen::analysis::type_checker::validate_types(&resolved)?;
    Ok(())
}

#[test]
fn test_negative_fixtures_rejected() {
    let dir = get_negative_dir();
    assert!(dir.exists(), "spec/negative must exist at {:?}", dir);

    let mut entries: Vec<_> = fs::read_dir(&dir)
        .expect("Failed to read spec negative dir")
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "alya"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    assert!(!entries.is_empty(), "spec/negative must not be empty");

    for entry in &entries {
        let filename = entry.file_name().to_string_lossy().to_string();
        let source = fs::read_to_string(entry.path()).unwrap();
        let (stage, fragment) = parse_expectation(&source);
        assert!(
            ["lex", "parse", "check"].contains(&stage.as_str()),
            "[{}] bad EXPECT-FAIL stage '{}'",
            filename,
            stage
        );

        let lex_result = Lexer::new(&source).tokenize();
        let tokens = match lex_result {
            Err(e) => {
                assert_eq!(
                    stage, "lex",
                    "[{}] expected {} failure but lexing failed",
                    filename, stage
                );
                assert!(
                    e.contains(&fragment),
                    "[{}] error message mismatch.\n  expected fragment: '{}'\n  actual: '{}'",
                    filename,
                    fragment,
                    e
                );
                println!("  [NEG OK] {:<40} ({}: {})", filename, stage, fragment);
                continue;
            }
            Ok(tokens) => {
                assert_ne!(
                    stage, "lex",
                    "[{}] expected lex failure but lexing succeeded",
                    filename
                );
                tokens
            }
        };
        let mut parser = Parser::new(tokens);
        let ast = match parser.parse() {
            Err(e) => {
                assert_eq!(
                    stage, "parse",
                    "[{}] expected {} failure but parsing failed",
                    filename, stage
                );
                assert!(
                    e.contains(&fragment),
                    "[{}] error message mismatch.\n  expected fragment: '{}'\n  actual: '{}'",
                    filename,
                    fragment,
                    e
                );
                println!("  [NEG OK] {:<40} ({}: {})", filename, stage, fragment);
                continue;
            }
            Ok(ast) => {
                assert_eq!(
                    stage, "check",
                    "[{}] expected {} failure but lex+parse succeeded",
                    filename, stage
                );
                ast
            }
        };
        let err = match run_check_stage(&ast, &dir) {
            Err(e) => e,
            Ok(_) => panic!(
                "[{}] expected check failure but full check passed (INVALID CODE ACCEPTED)",
                filename
            ),
        };
        assert!(
            err.contains(&fragment),
            "[{}] error message mismatch.\n  expected fragment: '{}'\n  actual: '{}'",
            filename,
            fragment,
            err
        );
        println!("  [NEG OK] {:<40} ({}: {})", filename, stage, fragment);
    }
}
