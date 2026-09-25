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

    if args.command == CommandKind::Bench {
        crate::tools::test_runner::run_benches(&args.input_file, args.arch, args.os)?;
        return Ok(());
    }

    if args.command == CommandKind::Repl {
        crate::tools::repl::start_repl(args.arch, args.os)?;
        return Ok(());
    }

    if args.command == CommandKind::Lsp {
        crate::tools::lsp::run_lsp()?;
        return Ok(());
    }

    if args.command == CommandKind::Dap {
        crate::tools::dap::run_dap()?;
        return Ok(());
    }

    if let CommandKind::Doc {
        ref input,
        ref output_dir,
        html,
        markdown,
    } = args.command
    {
        crate::tools::doc::run_doc(input, output_dir.as_deref(), html, markdown)?;
        return Ok(());
    }

    if let CommandKind::Lint {
        ref path,
        fix,
        check,
        format,
        ref output,
    } = args.command
    {
        crate::tools::lint::run_lint_cli(path.as_deref(), fix, check, format, output.as_deref())?;
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
    let imported_files =
        crate::parser::resolve_imports_with_sources_ext(&mut ast, base_dir, args.no_std)
            .map_err(|e| format!("Module import error in '{}': {}", args.input_file, e))?;
    let d_import = t_import.elapsed();

    if args.command == CommandKind::EmitAst {
        println!("{:#?}", ast);
        return Ok(());
    }

    // 3.5 Static Type Checking (Gradual Typing)
    let t_typecheck = Instant::now();
    let mut resolved_ast = ast.clone();
    crate::parser::enums::resolve_enums(&mut resolved_ast);
    crate::parser::constants::resolve_and_validate_constants(&mut resolved_ast)
        .map_err(|e| format!("Constant error in '{}': {}", args.input_file, e))?;
    crate::parser::generics::resolve_generics(&mut resolved_ast);
    let (type_warnings, type_result) =
        crate::codegen::analysis::type_checker::validate_types_with_warnings(&resolved_ast);
    for warning in type_warnings {
        eprintln!("{} in '{}'", warning, args.input_file);
    }
    type_result.map_err(|e| format!("Type error in '{}': {}", args.input_file, e))?;
    let d_typecheck = t_typecheck.elapsed();

    if args.command == CommandKind::Check {
        if !args.quiet {
            println!("✓ Syntax & Type Check OK: {}", args.input_file);
        }
        if args.time || args.stats {
            let total_dur = total_start.elapsed();
            println!("\n=== Check Profile: {} ===", args.input_file);
            println!(
                "  [Pass 1] Lexing:             {:>8.2} ms ({} tokens)",
                d_lex.as_secs_f64() * 1000.0,
                token_count
            );
            println!(
                "  [Pass 1] Parsing:            {:>8.2} ms ({} stmts)",
                d_parse.as_secs_f64() * 1000.0,
                stmt_count
            );
            println!(
                "  [Pass 2] Module Resolution:  {:>8.2} ms ({} files)",
                d_import.as_secs_f64() * 1000.0,
                imported_files.len()
            );
            println!(
                "  [Pass 3] Type Checking:      {:>8.2} ms",
                d_typecheck.as_secs_f64() * 1000.0
            );
            println!("  ----------------------------------------");
            println!(
                "  Total Check Time:            {:>8.2} ms",
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
            crate::tools::bundle::BundleOptions::new(default_stem, args.output_file.as_deref())
                .with_os(args.os.into())
                .with_arch(args.arch)
                .with_gui(args.gui);
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
                .binary_path()
                .with_extension("s")
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

    // 4. Code Generation (function inlining first, so DCE can prune
    // fully-inlined callees; checking already ran on source above)
    crate::parser::inline::inline_functions(&mut ast);
    let (code, pipeline_profile) =
        codegen::generate_full(&ast, args.arch, args.os, args.no_std, args.mem_trace);
    let asm_lines = code.lines().count();

    fs::write(&asm_file, code)
        .map_err(|e| format!("Error: Cannot write to '{}': {}", asm_file, e))?;

    let mut d_gcc = None;
    let mut d_exec = None;

    if let Some(exe_file) = final_output {
        if !args.quiet && args.command != CommandKind::Run {
            if bundle_opts.is_some() {
                println!("Compiling bundle binary: {}", exe_file);
            } else {
                println!("Compiling to executable: {}", exe_file);
            }
        }

        let t_gcc = Instant::now();
        let c_plan =
            c_builder::discover_c_build_plan(Path::new(&args.input_file), &imported_files)?;
        let mut c_objects = c_builder::build_c_objects(&c_plan, args.arch, args.os)?;
        // Windows bundles embed the staged icon as a COFF resource.
        // An embedded icon (.rsrc section) is unreferenced by code, so
        // linker section GC would silently drop it: keep all sections only
        // for icon bundles; every other link keeps --gc-sections.
        let mut with_icon_resource = false;
        if let Some(ref opts) = bundle_opts {
            if let Some(res) = opts.build_windows_icon_resource()? {
                c_objects.push(res);
                with_icon_resource = true;
            }
        }
        let mut extra_link_args: Vec<String> = Vec::new();
        if with_icon_resource {
            extra_link_args.push("-Wl,--no-gc-sections".to_string());
        }
        // Platform link flags from manifests (e.g. -lX11, -framework Cocoa).
        extra_link_args.extend(c_plan.link_flags_for(args.os));
        let mut extra_libs = codegen::collect_extern_libraries(&ast);
        extra_libs.retain(|lib| !c_plan.provided_libs.contains(lib));

        runner::compile_with_gcc(
            &asm_file,
            &exe_file,
            args.arch,
            args.os,
            &extra_libs,
            &c_objects,
            &extra_link_args,
        )?;
        d_gcc = Some(t_gcc.elapsed());

        if let Some(ref opts) = bundle_opts {
            if !args.quiet {
                use crate::tools::bundle::BundleOs;
                match opts.os {
                    BundleOs::MacOs => {
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
                    BundleOs::Windows => {
                        println!(
                            "✓ Successfully created Windows bundle: {}",
                            opts.bundle_dir.display()
                        );
                        println!("\nBundle contents:");
                        println!("  {}", opts.binary_path().display());
                        println!(
                            "  {}",
                            opts.bundle_dir
                                .join(format!("{}.exe.manifest", opts.app_name))
                                .display()
                        );
                    }
                    BundleOs::Linux => {
                        println!(
                            "✓ Successfully created Linux bundle: {}",
                            opts.bundle_dir.display()
                        );
                        println!("\nBundle contents:");
                        println!("  {}", opts.binary_path().display());
                        println!(
                            "  {}",
                            opts.bundle_dir
                                .join(format!("{}.desktop", opts.app_name))
                                .display()
                        );
                    }
                }
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
            use crate::tools::bundle::BundleOs;
            let kind = match opts.os {
                BundleOs::MacOs => "macOS App Bundle",
                BundleOs::Windows => "Windows bundle",
                BundleOs::Linux => "Linux bundle",
            };
            println!("✓ Generated {} assembly: {}", kind, asm_file);
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
        println!("  Source Size:                 {} bytes", source.len());
        println!(
            "  [Pass 1] Lexing:             {:>8.2} ms ({} tokens)",
            d_lex.as_secs_f64() * 1000.0,
            token_count
        );
        println!(
            "  [Pass 1] Parsing:            {:>8.2} ms ({} stmts)",
            d_parse.as_secs_f64() * 1000.0,
            stmt_count
        );
        println!(
            "  [Pass 2] Module Resolution:  {:>8.2} ms ({} files)",
            d_import.as_secs_f64() * 1000.0,
            imported_files.len()
        );
        println!(
            "  [Pass 3] CallIndex Build:    {:>8.2} ms",
            pipeline_profile.d_call_index.as_secs_f64() * 1000.0
        );
        println!(
            "  [Pass 4] Tree-Shaking (DCE): {:>8.2} ms ({}/{} stmts retained)",
            pipeline_profile.d_dce.as_secs_f64() * 1000.0,
            pipeline_profile.pruned_stmts,
            pipeline_profile.original_stmts
        );
        println!(
            "  [Pass 5] Type Inference:     {:>8.2} ms",
            pipeline_profile.d_inference.as_secs_f64() * 1000.0
        );
        println!(
            "  [Pass 6] Machine Codegen:    {:>8.2} ms ({} asm lines)",
            pipeline_profile.d_codegen.as_secs_f64() * 1000.0,
            asm_lines
        );
        if let Some(dur) = d_gcc {
            println!(
                "  [Pass 7] GCC Link:           {:>8.2} ms",
                dur.as_secs_f64() * 1000.0
            );
        }
        println!("  ----------------------------------------");
        if let Some(dur) = d_exec {
            println!(
                "  Compile Time:                {:>8.2} ms",
                (compile_time - dur).as_secs_f64() * 1000.0
            );
            println!(
                "  Execution Time:              {:>8.2} ms",
                dur.as_secs_f64() * 1000.0
            );
            println!(
                "  Total Time:                  {:>8.2} ms",
                compile_time.as_secs_f64() * 1000.0
            );
        } else {
            println!(
                "  Total Time:                  {:>8.2} ms",
                compile_time.as_secs_f64() * 1000.0
            );
        }
        println!("========================================\n");
    }

    Ok(())
}
