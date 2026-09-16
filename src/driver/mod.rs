use crate::cli::{CliArgs, CommandKind};
use crate::codegen::{self, Architecture, OperatingSystem};
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::fs;
use std::path::Path;
use std::time::Instant;
pub mod c_builder;
pub mod console;
pub mod runner;
pub mod toolchain;

pub use console::init_console;

pub fn run(args: CliArgs) -> Result<(), String> {
    init_console();

    if let CommandKind::Toolchain(ref tc_cmd) = args.command {
        toolchain::run_toolchain_cmd(tc_cmd, args.arch, args.os)?;
        return Ok(());
    }

    if let CommandKind::Pkg(ref pkg_cmd) = args.command {
        crate::tools::pkg::run_pkg(pkg_cmd)?;
        return Ok(());
    }

    if args.command == CommandKind::Fmt {
        crate::tools::fmt::run_fmt(&args.input_file, args.check_only).map(|_| ())?;
        return Ok(());
    }

    if args.command == CommandKind::Test {
        crate::tools::test_runner::run_tests(&args.input_file, args.arch, args.os, args.test_jobs)?;
        return Ok(());
    }

    if args.command == CommandKind::Repl {
        crate::tools::repl::start_repl(args.arch, args.os)?;
        return Ok(());
    }

    let total_start = Instant::now();

    let source = fs::read_to_string(&args.input_file)
        .map_err(|e| format!("Error: Cannot read file '{}': {}", args.input_file, e))?;

    // 1. Lexical Analysis
    let t_lex = Instant::now();
    let mut lexer = Lexer::new(&source);
    let tokens = lexer
        .tokenize()
        .map_err(|e| crate::diagnostics::render_error(&args.input_file, &source, &e))?;
    let d_lex = t_lex.elapsed();
    let token_count = tokens.len();

    if args.command == CommandKind::EmitTokens {
        println!("{:<12} {:<30}", "POSITION", "TOKEN");
        println!("{:-<12} {:-<30}", "", "");
        for t in &tokens {
            println!(
                "{:<12} {:?}",
                format!("{}:{}", t.line, t.column),
                t.token_type
            );
        }
        return Ok(());
    }

    // 2. Syntactic Analysis (Parsing)
    let t_parse = Instant::now();
    let mut parser = Parser::new(tokens);
    let mut ast = parser
        .parse()
        .map_err(|e| crate::diagnostics::render_error(&args.input_file, &source, &e))?;
    let d_parse = t_parse.elapsed();
    let stmt_count = ast.statements.len();

    // 3. Module Resolution
    let t_import = Instant::now();
    let base_dir = Path::new(&args.input_file)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let imported_files = crate::parser::resolve_imports_with_sources(&mut ast, base_dir)
        .map_err(|e| format!("Module import error in '{}': {}", args.input_file, e))?;
    let d_import = t_import.elapsed();

    if args.command == CommandKind::EmitAst {
        println!("{:#?}", ast);
        return Ok(());
    }

    if args.command == CommandKind::Check {
        if !args.quiet {
            println!("✓ Syntax OK: {}", args.input_file);
        }
        if args.time || args.stats {
            let total_dur = total_start.elapsed();
            println!("\n=== Check Profile: {} ===", args.input_file);
            println!(
                "  [1/3] Lexing:      {:>8.2} ms ({} tokens)",
                d_lex.as_secs_f64() * 1000.0,
                token_count
            );
            println!(
                "  [2/3] Parsing:     {:>8.2} ms ({} stmts)",
                d_parse.as_secs_f64() * 1000.0,
                stmt_count
            );
            println!(
                "  [3/3] Imports:     {:>8.2} ms",
                d_import.as_secs_f64() * 1000.0
            );
            println!("  ----------------------------------------");
            println!(
                "  Total Check Time:  {:>8.2} ms",
                total_dur.as_secs_f64() * 1000.0
            );
            println!("========================================");
        }
        return Ok(());
    }

    // Warnings for cross-compilation on Windows host
    if cfg!(target_os = "windows") {
        if matches!(args.arch, Architecture::ARM64) {
            eprintln!("Warning: ARM64 assembly generation is not supported on Windows MinGW/GCC.");
            eprintln!("         Windows GCC can only assemble x64 and x86 code.");
            eprintln!(
                "         The generated ARM64 assembly is valid but requires an ARM64 assembler."
            );
            eprintln!();
        }
        if matches!(args.arch, Architecture::X86) {
            eprintln!("Warning: x86 (32-bit) compilation requires gcc with multilib support.");
            eprintln!("         On Windows, you may need MinGW-w64 with 32-bit support.");
            eprintln!("         Try: gcc -m32 output.s -o program");
            eprintln!();
        }
    }

    // Determine smart base name from input file
    let default_stem = Path::new(&args.input_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let bundle_opts = if args.bundle {
        let mut opts =
            crate::tools::bundle::BundleOptions::new(default_stem, args.output_file.as_deref());
        opts.bundle_id = args.bundle_id.clone();
        opts.icon_path = args.icon_path.clone();
        opts.create_structure()?;
        Some(opts)
    } else {
        None
    };

    let is_binary = args.output_binary || args.command == CommandKind::Run;

    let (asm_file, final_output) = if let Some(ref opts) = bundle_opts {
        if is_binary {
            let temp_asm = format!("temp_{}_{}.s", default_stem, std::process::id());
            let exe_name = opts.binary_path().to_string_lossy().to_string();
            (temp_asm, Some(exe_name))
        } else {
            let asm_name = opts
                .bundle_dir
                .join("Contents")
                .join("MacOS")
                .join(format!("{}.s", opts.app_name))
                .to_string_lossy()
                .to_string();
            (asm_name, None)
        }
    } else if is_binary {
        let temp_asm = format!("temp_{}_{}.s", default_stem, std::process::id());
        let exe_name = args.output_file.clone().unwrap_or_else(|| {
            if args.command == CommandKind::Run {
                if matches!(args.os, OperatingSystem::Windows) {
                    format!("temp_{}_{}.exe", default_stem, std::process::id())
                } else {
                    format!("temp_{}_{}", default_stem, std::process::id())
                }
            } else if matches!(args.os, OperatingSystem::Windows) {
                format!("{}.exe", default_stem)
            } else {
                default_stem.to_string()
            }
        });
        (temp_asm, Some(exe_name))
    } else {
        let asm_name = args
            .output_file
            .clone()
            .unwrap_or_else(|| format!("{}.s", default_stem));
        (asm_name, None)
    };

    // 4. Code Generation
    let t_codegen = Instant::now();
    let code = codegen::generate(&ast, args.arch, args.os);
    let d_codegen = t_codegen.elapsed();
    let asm_lines = code.lines().count();

    fs::write(&asm_file, code)
        .map_err(|e| format!("Error: Cannot write to '{}': {}", asm_file, e))?;

    let mut d_gcc = None;
    let mut d_exec = None;

    if let Some(exe_file) = final_output {
        if !args.quiet && args.command != CommandKind::Run {
            if bundle_opts.is_some() {
                println!("Compiling macOS App Bundle binary: {}", exe_file);
            } else {
                println!("Compiling to executable: {}", exe_file);
            }
        }

        let t_gcc = Instant::now();
        let c_plan =
            c_builder::discover_c_build_plan(Path::new(&args.input_file), &imported_files)?;
        let c_objects = c_builder::build_c_objects(&c_plan, args.arch, args.os)?;
        let mut extra_libs = codegen::collect_extern_libraries(&ast);
        extra_libs.retain(|lib| !c_plan.provided_libs.contains(lib));

        runner::compile_with_gcc(
            &asm_file,
            &exe_file,
            args.arch,
            args.os,
            &extra_libs,
            &c_objects,
        )?;
        d_gcc = Some(t_gcc.elapsed());

        if let Some(ref opts) = bundle_opts {
            if !args.quiet {
                println!(
                    "✓ Successfully created macOS App Bundle: {}",
                    opts.bundle_dir.display()
                );
                println!("\nBundle contents:");
                println!(
                    "  {}",
                    opts.bundle_dir.join("Contents/Info.plist").display()
                );
                println!("  {}", opts.binary_path().display());
                println!(
                    "  {}",
                    opts.bundle_dir
                        .join("Contents/Resources/AppIcon.icns")
                        .display()
                );
                println!("\nTo launch on macOS:");
                println!("  open {}", opts.bundle_dir.display());
            }
        } else if args.command == CommandKind::Run {
            let t_exec = Instant::now();
            runner::execute_binary(&exe_file, &args.run_args, args.output_file.is_none())?;
            d_exec = Some(t_exec.elapsed());
        } else if !args.quiet {
            println!("✓ Successfully compiled to {}", exe_file);
            println!("\nRun your program:");
            if cfg!(target_os = "windows") {
                println!("  .\\{}", exe_file);
            } else {
                println!("  ./{}", exe_file);
            }
        }
    } else if let Some(ref opts) = bundle_opts {
        if !args.quiet {
            println!("✓ Generated macOS App Bundle assembly: {}", asm_file);
            println!("\nBundle contents:");
            println!(
                "  {}",
                opts.bundle_dir.join("Contents/Info.plist").display()
            );
            println!("  {}", asm_file);
            println!(
                "  {}",
                opts.bundle_dir
                    .join("Contents/Resources/AppIcon.icns")
                    .display()
            );
        }
    } else if !args.quiet {
        println!("Compiled successfully to {}", asm_file);
        println!("\nTo create executable:");
        println!("  gcc {} -o {} -no-pie", asm_file, default_stem);
        if cfg!(target_os = "windows") {
            println!("  .\\{}.exe", default_stem);
        } else {
            println!("  ./{}", default_stem);
        }
    }

    if args.time || args.stats {
        let compile_time = total_start.elapsed();
        println!("\n=== Compilation Profile: {} ===", args.input_file);
        println!("  Source Size:       {} bytes", source.len());
        println!(
            "  [1/5] Lexing:      {:>8.2} ms ({} tokens)",
            d_lex.as_secs_f64() * 1000.0,
            token_count
        );
        println!(
            "  [2/5] Parsing:     {:>8.2} ms ({} stmts)",
            d_parse.as_secs_f64() * 1000.0,
            stmt_count
        );
        println!(
            "  [3/5] Imports:     {:>8.2} ms",
            d_import.as_secs_f64() * 1000.0
        );
        println!(
            "  [4/5] Codegen:     {:>8.2} ms ({} asm lines)",
            d_codegen.as_secs_f64() * 1000.0,
            asm_lines
        );
        if let Some(dur) = d_gcc {
            println!(
                "  [5/5] GCC Link:    {:>8.2} ms",
                dur.as_secs_f64() * 1000.0
            );
        }
        println!("  ----------------------------------------");
        if let Some(dur) = d_exec {
            println!(
                "  Compile Time:      {:>8.2} ms",
                (compile_time - dur).as_secs_f64() * 1000.0
            );
            println!(
                "  Execution Time:    {:>8.2} ms",
                dur.as_secs_f64() * 1000.0
            );
            println!(
                "  Total Time:        {:>8.2} ms",
                compile_time.as_secs_f64() * 1000.0
            );
        } else {
            println!(
                "  Total Time:        {:>8.2} ms",
                compile_time.as_secs_f64() * 1000.0
            );
        }
        println!("========================================\n");
    }

    Ok(())
}
