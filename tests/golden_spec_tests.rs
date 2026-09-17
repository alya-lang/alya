use alya::lexer::Lexer;
use alya::parser::Parser;
use std::fs;
use std::path::PathBuf;

fn get_spec_syntax_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().join("spec").join("syntax")
}

#[test]
fn test_golden_spec_fixtures_count() {
    let syntax_dir = get_spec_syntax_dir();
    assert!(
        syntax_dir.exists(),
        "Spec syntax directory must exist at {:?}",
        syntax_dir
    );

    let alya_files: Vec<_> = fs::read_dir(&syntax_dir)
        .expect("Failed to read spec syntax dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "alya"))
        .collect();

    assert_eq!(
        alya_files.len(),
        25,
        "Expected exactly 25 canonical golden spec syntax files, but found {}",
        alya_files.len()
    );
}

#[test]
fn test_golden_spec_conformance_matrix() {
    let syntax_dir = get_spec_syntax_dir();
    let mut entries: Vec<_> = fs::read_dir(&syntax_dir)
        .expect("Failed to read spec syntax dir")
        .flatten()
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "alya"))
        .collect();

    entries.sort_by_key(|e| e.file_name());

    let mut lex_passed = 0;
    let mut parse_passed = 0;
    let total = entries.len();

    println!("\n=== ALYA SPEC v1.0 CONFORMANCE MATRIX ===");
    for entry in &entries {
        let filename = entry.file_name().to_string_lossy().to_string();
        let source = fs::read_to_string(entry.path()).unwrap();

        let mut lexer = Lexer::new(&source);
        let lex_res = lexer.tokenize();
        let (_lex_status, tokens_opt) = match lex_res {
            Ok(tokens) => {
                lex_passed += 1;
                ("PASS", Some(tokens))
            }
            Err(err) => {
                println!("  [LEX FAIL] {:<25} => {}", filename, err);
                ("FAIL", None)
            }
        };

        if let Some(tokens) = tokens_opt {
            let mut parser = Parser::new(tokens);
            match parser.parse() {
                Ok(_) => {
                    parse_passed += 1;
                    println!("  [PARSE OK] {:<25}", filename);
                }
                Err(err) => {
                    println!("  [PARSE FAIL] {:<23} => {}", filename, err);
                }
            }
        }
    }

    println!("=========================================");
    println!(
        "Summary: Lex: {}/{} passed, Parse: {}/{} passed\n",
        lex_passed, total, parse_passed, total
    );
}
