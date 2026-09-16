use super::*;

#[test]
fn test_parse_import() {
    let source = "import \"math_utils.alya\"\nsay 42";
    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    assert_eq!(program.statements.len(), 2);
    match &program.statements[0] {
        Stmt::Import { path, alias, .. } => {
            assert_eq!(path, "math_utils.alya");
            assert_eq!(alias, &None);
        }
        other => panic!("Expected Stmt::Import, got {:?}", other),
    }

    let source_alias = "import \"math_utils.alya\" as math\nsay math::add(1, 2)";
    let mut lexer = crate::lexer::Lexer::new(source_alias);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program_alias = parser.parse().expect("Failed to parse");
    match &program_alias.statements[0] {
        Stmt::Import { path, alias, .. } => {
            assert_eq!(path, "math_utils.alya");
            assert_eq!(alias, &Some("math".to_string()));
        }
        other => panic!("Expected Stmt::Import with alias, got {:?}", other),
    }
}

#[test]
fn test_resolve_imports_temporary_files() {
    use std::fs;
    let temp_dir = std::env::temp_dir().join(format!("alya_import_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    let helper_path = temp_dir.join("helper.alya");
    fs::write(&helper_path, "function get_val()\n    return 42\nend\n").unwrap();

    let main_source = "import \"helper.alya\"\nlet ans = get_val()\nsay ans";
    let mut lexer = crate::lexer::Lexer::new(main_source);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse().expect("Failed to parse");

    resolve_imports(&mut program, &temp_dir).expect("Failed to resolve imports");

    // After resolution, import is replaced by the function definition from helper.alya
    assert_eq!(program.statements.len(), 3);
    match &program.statements[0] {
        Stmt::Function { name, .. } => assert_eq!(name, "get_val"),
        other => panic!("Expected Stmt::Function, got {:?}", other),
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_parse_from_import() {
    let source = "from \"math_utils.alya\" import add, sub as subtract, *";
    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Stmt::Import {
            path,
            alias,
            symbols,
        } => {
            assert_eq!(path, "math_utils.alya");
            assert_eq!(alias, &None);
            let syms = symbols.as_ref().expect("Expected symbols");
            assert_eq!(syms.len(), 3);
            assert_eq!(syms[0].name, "add");
            assert_eq!(syms[0].alias, None);
            assert_eq!(syms[1].name, "sub");
            assert_eq!(syms[1].alias, Some("subtract".to_string()));
            assert_eq!(syms[2].name, "*");
        }
        other => panic!("Expected Stmt::Import, got {:?}", other),
    }
}

#[test]
fn test_resolve_from_import() {
    use std::fs;
    let temp_dir =
        std::env::temp_dir().join(format!("alya_from_import_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    let calc_path = temp_dir.join("calc.alya");
    fs::write(
        &calc_path,
        "function add(a, b)\n    return a + b\nend\nfunction sub(a, b)\n    return a - b\nend\n",
    )
    .unwrap();

    let main_source = "from \"calc.alya\" import add, sub as subtract\nlet x = add(10, 20)\nlet y = subtract(50, 15)";
    let mut lexer = crate::lexer::Lexer::new(main_source);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse().expect("Failed to parse");

    resolve_imports(&mut program, &temp_dir).expect("Failed to resolve imports");

    // Both add and subtract exist in statements
    let fns: Vec<String> = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Stmt::Function { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert!(fns.contains(&"add".to_string()));
    assert!(fns.contains(&"subtract".to_string()));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_resolve_imports_subdirectory_and_backslash_normalization() {
    use std::fs;
    let temp_dir =
        std::env::temp_dir().join(format!("alya_import_sub_test_{}", std::process::id()));
    let sub_dir = temp_dir.join("sub");
    let _ = fs::create_dir_all(&sub_dir);

    let helper_path = sub_dir.join("calc.alya");
    fs::write(
        &helper_path,
        "function calc_sum(a, b)\n    return a + b\nend\n",
    )
    .unwrap();

    // Test both forward slash and backslash in import path (duplicate is deduplicated)
    let main_source =
        "import \"sub/calc.alya\"\nimport \"sub\\\\calc.alya\"\nlet ans = calc_sum(1, 2)\nsay ans";
    let mut lexer = crate::lexer::Lexer::new(main_source);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse().expect("Failed to parse");

    resolve_imports(&mut program, &temp_dir).expect("Failed to resolve imports");

    // The function is imported and duplicate avoided
    assert_eq!(program.statements.len(), 3);
    match &program.statements[0] {
        Stmt::Function { name, .. } => assert_eq!(name, "calc_sum"),
        other => panic!("Expected Stmt::Function, got {:?}", other),
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_resolve_embedded_stdlib_modules() {
    let code = r#"
import "std/math"
import "std/time"
import "std/os"
import "std/json"
import "std/mem"
import "std/str"
import "std/path"
import "std/fs"
import "std/hash"
import "std/collections"
import "std/test"
import "std/rand"
import "std/cli"
import "std/color"
import "std/log"
import "std/glob"
import "std/thread"
say PI
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().expect("Tokenize failed");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parse failed");

    // Use a non-existent directory to force fallback to embedded stdlib
    let dummy_dir = std::path::Path::new("non_existent_dir_for_test");
    resolve_imports(&mut ast, dummy_dir).expect("Embedded stdlib resolution should succeed");

    // Check that functions and constants from stdlib were imported
    let has_hypot = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "hypot",
        _ => false,
    });
    let has_now = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "now",
        _ => false,
    });
    let has_env = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "env",
        _ => false,
    });
    let has_json_bool = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "json_bool",
        _ => false,
    });
    let has_arena_new = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "arena_new",
        _ => false,
    });
    let has_starts_with = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "starts_with",
        _ => false,
    });
    let has_path_join = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "path_join",
        _ => false,
    });
    let has_copy_file = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "copy_file",
        _ => false,
    });
    let has_fnv1a = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "fnv1a",
        _ => false,
    });
    let has_stack_new = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "stack_new",
        _ => false,
    });
    let has_assert_eq = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "assert_eq",
        _ => false,
    });

    let has_rand_int = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "rand_int",
        _ => false,
    });

    let has_rand_float = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "rand_float",
        _ => false,
    });

    let has_cli_has_flag = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "cli_has_flag",
        _ => false,
    });

    let has_color_red = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "color_red",
        _ => false,
    });

    let has_logger_new = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "logger_new",
        _ => false,
    });

    let has_glob_match = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "glob_match",
        _ => false,
    });
    let has_thread_spawn = ast.statements.iter().any(|s| match s {
        Stmt::Function { name, .. } => name == "thread_spawn",
        _ => false,
    });

    assert!(has_hypot, "Missing hypot from std/math");
    assert!(has_now, "Missing now from std/time");
    assert!(has_env, "Missing env from std/os");
    assert!(has_json_bool, "Missing json_bool from std/json");
    assert!(has_arena_new, "Missing arena_new from std/mem");
    assert!(has_starts_with, "Missing starts_with from std/str");
    assert!(has_path_join, "Missing path_join from std/path");
    assert!(has_copy_file, "Missing copy_file from std/fs");
    assert!(has_fnv1a, "Missing fnv1a from std/hash");
    assert!(has_stack_new, "Missing stack_new from std/collections");
    assert!(has_assert_eq, "Missing assert_eq from std/test");
    assert!(has_rand_int, "Missing rand_int from std/rand");
    assert!(has_rand_float, "Missing rand_float from std/rand");
    assert!(has_cli_has_flag, "Missing cli_has_flag from std/cli");
    assert!(has_color_red, "Missing color_red from std/color");
    assert!(has_logger_new, "Missing logger_new from std/log");
    assert!(has_glob_match, "Missing glob_match from std/glob");
    assert!(has_thread_spawn, "Missing thread_spawn from std/thread");
}

#[test]
fn test_embedded_stdlib_deduplication() {
    let code = r#"
import "std/math"
import "std/math"
say PI
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().expect("Tokenize failed");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parse failed");

    let dummy_dir = std::path::Path::new("non_existent_dir_for_test");
    resolve_imports(&mut ast, dummy_dir).expect("Embedded stdlib resolution should succeed");

    let hypot_count = ast
        .statements
        .iter()
        .filter(|s| match s {
            Stmt::Function { name, .. } => name == "hypot",
            _ => false,
        })
        .count();

    assert_eq!(
        hypot_count, 1,
        "Duplicate import of std/math should only include hypot once"
    );
}

#[test]
fn test_import_with_alias_resolution() {
    use std::fs;
    let temp_dir = std::env::temp_dir().join(format!("alya_alias_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    let helper_path = temp_dir.join("calc.alya");
    fs::write(
        &helper_path,
        "function mult(a, b)\n    return a * b\nend\nfunction square(x)\n    return mult(x, x)\nend\n",
    )
    .unwrap();

    let main_code = "import \"calc.alya\" as c\nlet ans = c::square(5)\nsay ans";
    let mut lexer = Lexer::new(main_code);
    let tokens = lexer.tokenize().expect("Tokenize failed");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parse failed");

    resolve_imports(&mut ast, &temp_dir).expect("Resolve imports should succeed");

    // Check that functions are prefixed with c::
    let fn_names: Vec<String> = ast
        .statements
        .iter()
        .filter_map(|s| match s {
            Stmt::Function { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();

    assert!(fn_names.contains(&"c::mult".to_string()));
    assert!(fn_names.contains(&"c::square".to_string()));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_import_modular_submodules_with_alias_resolution() {
    use std::fs;
    let temp_dir =
        std::env::temp_dir().join(format!("alya_modular_alias_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    let helper_path = temp_dir.join("core.alya");
    fs::write(
        &helper_path,
        "function core_calc(x)\n    return x * 10\nend\n",
    )
    .unwrap();

    let lib_path = temp_dir.join("lib.alya");
    fs::write(
        &lib_path,
        "import \"./core.alya\"\nfunction lib_run(x)\n    return core_calc(x) + 1\nend\n",
    )
    .unwrap();

    let main_code = "import \"lib.alya\" as mymod\nlet res = mymod::lib_run(5)\nsay res";
    let mut lexer = Lexer::new(main_code);
    let tokens = lexer.tokenize().expect("Tokenize failed");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parse failed");

    resolve_imports(&mut ast, &temp_dir).expect("Resolve imports should succeed");

    let fn_names: Vec<String> = ast
        .statements
        .iter()
        .filter_map(|s| match s {
            Stmt::Function { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();

    assert!(fn_names.contains(&"mymod::core_calc".to_string()));
    assert!(fn_names.contains(&"mymod::lib_run".to_string()));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_duplicate_function_definition_error() {
    use std::fs;
    let temp_dir = std::env::temp_dir().join(format!("alya_dup_err_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    let m1_path = temp_dir.join("m1.alya");
    fs::write(&m1_path, "function abc()\n    return 1\nend\n").unwrap();

    let m2_path = temp_dir.join("m2.alya");
    fs::write(&m2_path, "function abc()\n    return 2\nend\n").unwrap();

    let main_code = "import \"m1.alya\"\nimport \"m2.alya\"\nsay abc()";
    let mut lexer = Lexer::new(main_code);
    let tokens = lexer.tokenize().expect("Tokenize failed");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parse failed");

    let res = resolve_imports(&mut ast, &temp_dir);
    assert!(
        res.is_err(),
        "Duplicate function abc should fail resolution"
    );
    let err_msg = res.unwrap_err();
    assert!(
        err_msg.contains("Duplicate function definition 'abc'"),
        "Error message should mention duplicate function definition 'abc', got: {}",
        err_msg
    );
    assert!(
        err_msg.contains("use 'import \"...\" as <alias>'"),
        "Error message should advise using alias, got: {}",
        err_msg
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_aliased_import_resolves_conflict() {
    use std::fs;
    let temp_dir = std::env::temp_dir().join(format!("alya_alias_resolve_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    let m1_path = temp_dir.join("m1.alya");
    fs::write(&m1_path, "function abc()\n    return 1\nend\n").unwrap();

    let m2_path = temp_dir.join("m2.alya");
    fs::write(&m2_path, "function abc()\n    return 2\nend\n").unwrap();

    let main_code =
        "import \"m1.alya\" as one\nimport \"m2.alya\" as two\nsay one::abc()\nsay two::abc()";
    let mut lexer = Lexer::new(main_code);
    let tokens = lexer.tokenize().expect("Tokenize failed");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parse failed");

    let res = resolve_imports(&mut ast, &temp_dir);
    assert!(
        res.is_ok(),
        "Aliased imports should resolve conflict successfully"
    );

    let fn_names: Vec<String> = ast
        .statements
        .iter()
        .filter_map(|s| match s {
            Stmt::Function { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();

    assert!(fn_names.contains(&"one::abc".to_string()));
    assert!(fn_names.contains(&"two::abc".to_string()));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_import_embedded_color_and_log_stdlib() {
    let source = "import \"std/color\"\nimport \"std/log\"\nlog_info(\"test\")";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenize failed");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parse failed");

    let current_dir = std::path::Path::new(".");
    let res = resolve_imports(&mut ast, current_dir);
    assert!(
        res.is_ok(),
        "Importing std/color and std/log should succeed"
    );

    let fn_names: Vec<String> = ast
        .statements
        .iter()
        .filter_map(|s| match s {
            Stmt::Function { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();

    assert!(fn_names.contains(&"color_green".to_string()));
    assert!(fn_names.contains(&"ansi_strip".to_string()));
    assert!(fn_names.contains(&"logger_new".to_string()));
    assert!(fn_names.contains(&"log_info".to_string()));
}

#[test]
fn test_resolve_package_import_via_manifest() {
    let base_temp = std::env::temp_dir().join(format!(
        "alya_parser_pkg_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let lib_dir = base_temp.join("mylib");
    let app_dir = base_temp.join("myapp");

    let _ = std::fs::create_dir_all(&lib_dir);
    let _ = std::fs::create_dir_all(&app_dir);

    // Create library file
    std::fs::write(
        lib_dir.join("main.alya"),
        "function compute_cube(x)\n    return x * x * x\nend\n",
    )
    .unwrap();

    // Create app alya.toml
    let toml = format!(
        "[package]\nname = \"myapp\"\nversion = \"0.1.0\"\nentry = \"src/main.alya\"\n\n[dependencies]\nmylib = {{ path = \"{}\" }}\n",
        lib_dir.to_string_lossy().replace('\\', "/")
    );
    std::fs::write(app_dir.join("alya.toml"), toml).unwrap();

    let source = "import \"mylib\" as math\nlet res = math::compute_cube(3)";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenize failed");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parse failed");

    let res = resolve_imports(&mut ast, &app_dir);
    assert!(
        res.is_ok(),
        "Importing package defined in alya.toml should succeed: {:?}",
        res
    );

    let fn_names: Vec<String> = ast
        .statements
        .iter()
        .filter_map(|s| match s {
            Stmt::Function { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();

    assert!(fn_names.contains(&"math::compute_cube".to_string()));

    let _ = std::fs::remove_dir_all(&base_temp);
}

#[test]
fn test_resolve_uninstalled_package_error() {
    let base_temp = std::env::temp_dir().join(format!(
        "alya_parser_missing_pkg_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let app_dir = base_temp.join("myapp");
    let _ = std::fs::create_dir_all(&app_dir);

    let toml = "[package]\nname = \"myapp\"\nversion = \"0.1.0\"\nentry = \"src/main.alya\"\n\n[dependencies]\nsqlite = \"1.0.0\"\n";
    std::fs::write(app_dir.join("alya.toml"), toml).unwrap();

    let source = "import \"sqlite\"\nsay \"hello\"";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Tokenize failed");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parse failed");

    let res = resolve_imports(&mut ast, &app_dir);
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    assert!(err_msg.contains("Run 'alyac install' to resolve dependencies."));

    let _ = std::fs::remove_dir_all(&base_temp);
}

#[test]
fn test_deprecated_stdlib_modules_diagnostic() {
    let code_csv = "import \"std/csv\"\nsay 1";
    let mut ast = Parser::new(Lexer::new(code_csv).tokenize().unwrap())
        .parse()
        .unwrap();
    let err = resolve_imports(&mut ast, std::path::Path::new(".")).unwrap_err();
    assert!(err.contains("Cannot find standard library module 'std/csv'"));

    let code_url = "import \"std/url\"\nsay 1";
    let mut ast2 = Parser::new(Lexer::new(code_url).tokenize().unwrap())
        .parse()
        .unwrap();
    let err2 = resolve_imports(&mut ast2, std::path::Path::new(".")).unwrap_err();
    assert!(err2.contains("Cannot find standard library module 'std/url'"));

    let code_crypto = "import \"std/crypto\"\nsay 1";
    let mut ast_crypto = Parser::new(Lexer::new(code_crypto).tokenize().unwrap())
        .parse()
        .unwrap();
    let err_crypto = resolve_imports(&mut ast_crypto, std::path::Path::new(".")).unwrap_err();
    assert!(err_crypto.contains("Cannot find standard library module 'std/crypto'"));
}

#[test]
fn test_transitive_and_direct_stdlib_import_private_deduplication() {
    let code = r#"
import "std/color" as c
import "std/color"
say c::color_red("hello")
say color_blue("world")
"#;
    let mut ast = Parser::new(Lexer::new(code).tokenize().unwrap())
        .parse()
        .unwrap();
    let res = resolve_imports(&mut ast, std::path::Path::new("."));
    assert!(res.is_ok(), "Import resolution failed: {:?}", res);

    let fn_names: Vec<String> = ast
        .statements
        .iter()
        .filter_map(|s| match s {
            Stmt::Function { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();

    // Verify both aliased and unaliased public functions exist
    assert!(fn_names.iter().any(|n| n == "c::color_red"));
    assert!(fn_names.iter().any(|n| n == "color_red"));

    // Verify private helper _esc was deduplicated and only defined once
    let esc_count = fn_names.iter().filter(|n| n.contains("::_esc")).count();
    assert_eq!(
        esc_count, 1,
        "Expected exactly 1 _esc definition, found {}",
        esc_count
    );
}
