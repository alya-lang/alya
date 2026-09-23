use crate::codegen::{Architecture, OperatingSystem};
use crate::driver::toolchain::ToolchainCommand;
use crate::tools::pkg::PkgCommand;
use std::env;
use std::process;

/// Host platform as an `OperatingSystem`: bundle commands default to it.
pub(crate) fn host_operating_system() -> OperatingSystem {
    if cfg!(target_os = "windows") {
        OperatingSystem::Windows
    } else if cfg!(target_os = "macos") {
        OperatingSystem::MacOS
    } else {
        OperatingSystem::Linux
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandKind {
    Build,
    Run,
    Check,
    EmitTokens,
    EmitAst,
    Fmt,
    Test,
    Bench,
    Repl,
    Pkg(PkgCommand),
    Toolchain(ToolchainCommand),
    Lsp,
    Dap,
    Doc {
        input: String,
        output_dir: Option<String>,
        html: bool,
        markdown: bool,
    },
    Lint {
        path: Option<String>,
        fix: bool,
        check: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliArgs {
    pub command: CommandKind,
    pub input_file: String,
    pub output_file: Option<String>,
    pub output_binary: bool,
    pub arch: Architecture,
    pub os: OperatingSystem,
    pub quiet: bool,
    pub time: bool,
    pub stats: bool,
    pub check_only: bool,
    pub bundle: bool,
    pub bundle_id: Option<String>,
    pub icon_path: Option<String>,
    pub gui: bool,
    pub run_args: Vec<String>,
    pub test_jobs: Option<usize>,
    pub no_std: bool,
    pub mem_trace: bool,
}

impl CliArgs {
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        match Self::parse_from(&args) {
            Ok(Some(parsed)) => parsed,
            Ok(None) => process::exit(0),
            Err(err) => {
                eprintln!("{}", err);
                eprintln!("Run 'alya --help' for usage instructions.");
                process::exit(1);
            }
        }
    }

    pub fn parse_from(args: &[String]) -> Result<Option<Self>, String> {
        if args.len() < 2 {
            let arch = if cfg!(target_arch = "aarch64") {
                Architecture::ARM64
            } else if cfg!(target_arch = "x86") {
                Architecture::X86
            } else {
                Architecture::X64
            };
            let os = if cfg!(target_os = "windows") {
                OperatingSystem::Windows
            } else if cfg!(target_os = "macos") {
                OperatingSystem::MacOS
            } else {
                OperatingSystem::Linux
            };
            return Ok(Some(Self {
                command: CommandKind::Repl,
                input_file: String::new(),
                output_file: None,
                output_binary: false,
                arch,
                os,
                quiet: false,
                time: false,
                stats: false,
                check_only: false,
                bundle: false,
                gui: false,
                bundle_id: None,
                icon_path: None,
                run_args: Vec::new(),
                test_jobs: None,
                no_std: false,
                mem_trace: false,
            }));
        }

        let first = args[1].as_str();
        if first == "-h" || first == "--help" || first == "help" {
            Self::print_usage();
            return Ok(None);
        }
        if first == "-v" || first == "--version" || first == "version" {
            Self::print_version();
            return Ok(None);
        }

        if first == "init" {
            let pkg_cmd = parse_pkg_init_args(&args[2..])?;
            return Ok(Some(Self::create_pkg_args(pkg_cmd)));
        }
        if first == "add" {
            let pkg_cmd = parse_pkg_add_args(&args[2..])?;
            return Ok(Some(Self::create_pkg_args(pkg_cmd)));
        }
        if first == "install" {
            return Ok(Some(Self::create_pkg_args(PkgCommand::Install)));
        }
        if first == "cache" {
            let pkg_cmd = parse_pkg_cache_args(&args[2..])?;
            return Ok(Some(Self::create_pkg_args(pkg_cmd)));
        }
        if first == "clean" {
            let pkg_cmd = parse_pkg_clean_args(&args[2..])?;
            return Ok(Some(Self::create_pkg_args(pkg_cmd)));
        }
        if first == "update" {
            let pkg_cmd = parse_pkg_update_args(&args[2..])?;
            return Ok(Some(Self::create_pkg_args(pkg_cmd)));
        }
        if first == "outdated" {
            return Ok(Some(Self::create_pkg_args(PkgCommand::Update {
                upgrade: false,
            })));
        }
        if first == "pkg" {
            if args.len() < 3 || args[2] == "-h" || args[2] == "--help" || args[2] == "help" {
                return Ok(Some(Self::create_pkg_args(PkgCommand::Help)));
            }
            let sub = args[2].as_str();
            let pkg_cmd = match sub {
                "init" => parse_pkg_init_args(&args[3..])?,
                "add" => parse_pkg_add_args(&args[3..])?,
                "install" => PkgCommand::Install,
                "list" => PkgCommand::List,
                "update" => parse_pkg_update_args(&args[3..])?,
                "outdated" => PkgCommand::Update { upgrade: false },
                "cache" => parse_pkg_cache_args(&args[3..])?,
                "clean" => parse_pkg_clean_args(&args[3..])?,
                "help" | "-h" | "--help" => PkgCommand::Help,
                other => {
                    return Err(format!(
                        "Error: Unknown pkg subcommand '{}'. Run 'alya pkg help' for usage.",
                        other
                    ))
                }
            };
            return Ok(Some(Self::create_pkg_args(pkg_cmd)));
        }

        if first == "toolchain" {
            let tc_cmd = parse_toolchain_args(&args[2..])?;
            return Ok(Some(Self::create_toolchain_args(tc_cmd)));
        }

        if first == "lsp" {
            return Ok(Some(Self::create_simple_args(CommandKind::Lsp)));
        }

        if first == "dap" {
            return Ok(Some(Self::create_simple_args(CommandKind::Dap)));
        }

        if first == "doc" {
            let doc_cmd = parse_doc_args(&args[2..])?;
            return Ok(Some(Self::create_simple_args(doc_cmd)));
        }

        if first == "lint" {
            let lint_cmd = parse_lint_args(&args[2..])?;
            return Ok(Some(Self::create_simple_args(lint_cmd)));
        }

        let mut command = CommandKind::Build;
        let mut output_binary = false;
        let mut start_idx = 1;

        match first {
            "repl" => {
                command = CommandKind::Repl;
                start_idx = 2;
            }
            "run" => {
                command = CommandKind::Run;
                output_binary = true;
                start_idx = 2;
            }
            "build" => {
                command = CommandKind::Build;
                output_binary = true;
                start_idx = 2;
            }
            "check" => {
                command = CommandKind::Check;
                start_idx = 2;
            }
            "ast" => {
                command = CommandKind::EmitAst;
                start_idx = 2;
            }
            "tokens" => {
                command = CommandKind::EmitTokens;
                start_idx = 2;
            }
            "fmt" => {
                command = CommandKind::Fmt;
                start_idx = 2;
            }
            "test" => {
                command = CommandKind::Test;
                start_idx = 2;
            }
            "bench" => {
                command = CommandKind::Bench;
                start_idx = 2;
            }
            _ => {}
        }

        let mut input_file = None;
        let mut output_file = None;
        let mut quiet = false;
        let mut time = false;
        let mut stats = false;
        let mut check_only = false;
        let mut bundle = false;
        let mut bundle_id = None;
        let mut icon_path = None;
        let mut gui = false;
        let mut os_explicit = false;
        let mut arch_explicit = false;
        let mut run_args = Vec::new();
        let mut test_jobs = None;
        let mut no_std = false;
        let mut mem_trace = false;
        let mut arch = if cfg!(target_arch = "aarch64") {
            Architecture::ARM64
        } else if cfg!(target_arch = "x86") {
            Architecture::X86
        } else {
            Architecture::X64
        };
        let mut os = if cfg!(target_os = "windows") {
            OperatingSystem::Windows
        } else if cfg!(target_os = "macos") {
            OperatingSystem::MacOS
        } else {
            OperatingSystem::Linux
        };

        let mut i = start_idx;
        while i < args.len() {
            match args[i].as_str() {
                "--" => {
                    run_args.extend(args[i + 1..].iter().cloned());
                    break;
                }
                "-h" | "--help" => {
                    Self::print_usage();
                    return Ok(None);
                }
                "-v" | "--version" => {
                    Self::print_version();
                    return Ok(None);
                }
                "-o" | "--output" => {
                    if i + 1 < args.len() {
                        output_file = Some(args[i + 1].clone());
                        i += 1;
                    } else {
                        return Err("Error: Missing argument for '-o/--output'".to_string());
                    }
                }
                "-b" | "-c" | "--binary" | "--output-binary" => {
                    output_binary = true;
                }
                "-S" | "--asm" => {
                    output_binary = false;
                }
                "-r" | "--run" => {
                    command = CommandKind::Run;
                    output_binary = true;
                }
                "--check" => {
                    if command == CommandKind::Fmt {
                        check_only = true;
                    } else {
                        command = CommandKind::Check;
                    }
                }
                "--ast" => {
                    command = CommandKind::EmitAst;
                }
                "--tokens" => {
                    command = CommandKind::EmitTokens;
                }
                "-q" | "--quiet" => {
                    quiet = true;
                }
                "--sequential" => {
                    test_jobs = Some(1);
                }
                "-j" | "--jobs" | "--test-threads" => {
                    if i + 1 < args.len() {
                        let val = args[i + 1]
                            .parse::<usize>()
                            .map_err(|_| format!("Error: Invalid jobs count '{}'", args[i + 1]))?;
                        test_jobs = Some(val.max(1));
                        i += 1;
                    } else {
                        return Err("Error: Missing argument for '-j/--jobs'".to_string());
                    }
                }
                "--time" => {
                    time = true;
                }
                "--no-std" => {
                    no_std = true;
                }
                "--mem-trace" => {
                    mem_trace = true;
                }
                "--stats" | "--bench" => {
                    stats = true;
                    time = true;
                }
                "--bundle" | "--app" => {
                    bundle = true;
                    output_binary = true;
                }
                "--gui" => {
                    bundle = true;
                    output_binary = true;
                    gui = true;
                }
                "--bundle-id" | "--identifier" => {
                    if i + 1 < args.len() {
                        bundle_id = Some(args[i + 1].clone());
                        i += 1;
                    } else {
                        return Err("Error: Missing argument for '--bundle-id'".to_string());
                    }
                }
                "--icon" => {
                    if i + 1 < args.len() {
                        icon_path = Some(args[i + 1].clone());
                        i += 1;
                    } else {
                        return Err("Error: Missing argument for '--icon'".to_string());
                    }
                }
                "--arch" => {
                    if i + 1 < args.len() {
                        arch_explicit = true;
                        arch = match args[i + 1].as_str() {
                            "x64" => Architecture::X64,
                            "x86" => Architecture::X86,
                            "arm64" => Architecture::ARM64,
                            other => {
                                return Err(format!(
                                    "Error: Unknown architecture '{}'. Supported: x86, x64, arm64",
                                    other
                                ))
                            }
                        };
                        i += 1;
                    } else {
                        return Err("Error: Missing argument for '--arch'".to_string());
                    }
                }
                "--os" => {
                    if i + 1 < args.len() {
                        os_explicit = true;
                        os = match args[i + 1].as_str() {
                            "linux" => OperatingSystem::Linux,
                            "windows" => OperatingSystem::Windows,
                            "macos" => OperatingSystem::MacOS,
                            other => {
                                return Err(format!(
                                    "Error: Unknown OS '{}'. Supported: linux, windows, macos",
                                    other
                                ))
                            }
                        };
                        i += 1;
                    } else {
                        return Err("Error: Missing argument for '--os'".to_string());
                    }
                }
                arg if !arg.starts_with('-') => {
                    if let Some(existing) = &input_file {
                        if command == CommandKind::Run {
                            run_args.push(arg.to_string());
                        } else {
                            return Err(format!(
                                "Error: Unexpected multiple input files: '{}' and '{}'",
                                existing, arg
                            ));
                        }
                    } else {
                        input_file = Some(arg.to_string());
                    }
                }
                other => {
                    if command == CommandKind::Run && input_file.is_some() {
                        run_args.push(other.to_string());
                    } else {
                        return Err(format!("Error: Unknown option '{}'", other));
                    }
                }
            }
            i += 1;
        }

        let input_file = match input_file {
            Some(f) => f,
            None => {
                if matches!(
                    command,
                    CommandKind::Fmt | CommandKind::Test | CommandKind::Bench
                ) {
                    ".".to_string()
                } else if command == CommandKind::Repl || matches!(command, CommandKind::Pkg(_)) {
                    String::new()
                } else if matches!(
                    command,
                    CommandKind::Run | CommandKind::Build | CommandKind::Check
                ) {
                    if let Some(entry) = crate::tools::pkg::detect_package_entry() {
                        entry
                    } else {
                        return Err("Error: No input source file specified.".to_string());
                    }
                } else {
                    return Err("Error: No input source file specified.".to_string());
                }
            }
        };

        if bundle {
            // Default to the host platform so `alya build app.alya --gui`
            // yields a runnable bundle where it runs; cross-bundle with
            // an explicit `--os windows|linux|macos`.
            if !os_explicit {
                os = host_operating_system();
            }
            if !arch_explicit && !matches!(arch, Architecture::ARM64 | Architecture::X64) {
                arch = Architecture::ARM64;
            }
        }

        Ok(Some(Self {
            command,
            input_file,
            output_file,
            output_binary,
            arch,
            os,
            quiet,
            time,
            stats,
            check_only,
            bundle,
            bundle_id,
            icon_path,
            gui,
            run_args,
            test_jobs,
            no_std,
            mem_trace,
        }))
    }

    fn create_pkg_args(pkg_cmd: PkgCommand) -> Self {
        let arch = if cfg!(target_arch = "aarch64") {
            Architecture::ARM64
        } else if cfg!(target_arch = "x86") {
            Architecture::X86
        } else {
            Architecture::X64
        };
        let os = if cfg!(target_os = "windows") {
            OperatingSystem::Windows
        } else if cfg!(target_os = "macos") {
            OperatingSystem::MacOS
        } else {
            OperatingSystem::Linux
        };
        Self {
            command: CommandKind::Pkg(pkg_cmd),
            input_file: String::new(),
            output_file: None,
            output_binary: false,
            arch,
            os,
            quiet: false,
            time: false,
            stats: false,
            check_only: false,
            bundle: false,
            gui: false,
            bundle_id: None,
            icon_path: None,
            run_args: Vec::new(),
            test_jobs: None,
            no_std: false,
            mem_trace: false,
        }
    }

    fn create_toolchain_args(tc_cmd: ToolchainCommand) -> Self {
        let arch = if cfg!(target_arch = "aarch64") {
            Architecture::ARM64
        } else if cfg!(target_arch = "x86") {
            Architecture::X86
        } else {
            Architecture::X64
        };
        let os = if cfg!(target_os = "windows") {
            OperatingSystem::Windows
        } else if cfg!(target_os = "macos") {
            OperatingSystem::MacOS
        } else {
            OperatingSystem::Linux
        };
        Self {
            command: CommandKind::Toolchain(tc_cmd),
            input_file: String::new(),
            output_file: None,
            output_binary: false,
            arch,
            os,
            quiet: false,
            time: false,
            stats: false,
            check_only: false,
            bundle: false,
            gui: false,
            bundle_id: None,
            icon_path: None,
            run_args: Vec::new(),
            test_jobs: None,
            no_std: false,
            mem_trace: false,
        }
    }

    fn create_simple_args(command: CommandKind) -> Self {
        let arch = if cfg!(target_arch = "aarch64") {
            Architecture::ARM64
        } else if cfg!(target_arch = "x86") {
            Architecture::X86
        } else {
            Architecture::X64
        };
        let os = if cfg!(target_os = "windows") {
            OperatingSystem::Windows
        } else if cfg!(target_os = "macos") {
            OperatingSystem::MacOS
        } else {
            OperatingSystem::Linux
        };
        Self {
            command,
            input_file: String::new(),
            output_file: None,
            output_binary: false,
            arch,
            os,
            quiet: false,
            time: false,
            stats: false,
            check_only: false,
            bundle: false,
            gui: false,
            bundle_id: None,
            icon_path: None,
            run_args: Vec::new(),
            test_jobs: None,
            no_std: false,
            mem_trace: false,
        }
    }

    pub fn print_version() {
        crate::cli::help::print_version();
    }

    pub fn print_usage() {
        crate::cli::help::print_usage();
    }
}

fn parse_doc_args(args: &[String]) -> Result<CommandKind, String> {
    let mut input = None;
    let mut output_dir = None;
    let mut html = false;
    let mut markdown = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("Usage: alya doc [path] [options]");
                println!();
                println!("Options:");
                println!(
                    "  -o, --output <dir>   Output directory for generated docs (default: docs)"
                );
                println!("  --html               Generate HTML documentation");
                println!("  --md, --markdown     Generate Markdown documentation");
                println!("  -h, --help           Show help");
                process::exit(0);
            }
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_dir = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    return Err("Error: Missing argument for '-o/--output'".to_string());
                }
            }
            "--html" => {
                html = true;
            }
            "--md" | "--markdown" => {
                markdown = true;
            }
            other if !other.starts_with('-') => {
                if input.is_none() {
                    input = Some(other.to_string());
                } else {
                    return Err(format!("Error: Unexpected argument '{}'", other));
                }
            }
            other => {
                return Err(format!("Error: Unknown doc option '{}'", other));
            }
        }
        i += 1;
    }

    let input_path = input.unwrap_or_else(|| {
        if std::path::Path::new("src").is_dir() {
            "src".to_string()
        } else {
            ".".to_string()
        }
    });

    Ok(CommandKind::Doc {
        input: input_path,
        output_dir,
        html,
        markdown,
    })
}

fn parse_lint_args(args: &[String]) -> Result<CommandKind, String> {
    let mut path = None;
    let mut fix = false;
    let mut check = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("Usage: alya lint [path] [options]");
                println!();
                println!("Performs static semantic analysis and code smell detection.");
                println!();
                println!("Options:");
                println!("  --fix      Automatically refactor and clean up safe warnings in-place");
                println!("  --check    Exit with non-zero status if warnings are detected (CI quality gate)");
                println!("  -h, --help Show help");
                process::exit(0);
            }
            "--fix" => {
                fix = true;
            }
            "--check" => {
                check = true;
            }
            other if !other.starts_with('-') => {
                if path.is_none() {
                    path = Some(other.to_string());
                } else {
                    return Err(format!("Error: Unexpected argument '{}'", other));
                }
            }
            other => {
                return Err(format!("Error: Unknown lint option '{}'", other));
            }
        }
        i += 1;
    }

    Ok(CommandKind::Lint { path, fix, check })
}

fn parse_pkg_init_args(args: &[String]) -> Result<PkgCommand, String> {
    let mut path = None;
    let mut name = None;
    let mut is_lib = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--lib" => is_lib = true,
            "--name" => {
                if i + 1 < args.len() {
                    name = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    return Err("Error: Missing argument for '--name'".to_string());
                }
            }
            arg if !arg.starts_with('-') => {
                if path.is_none() {
                    path = Some(arg.to_string());
                } else {
                    return Err(format!("Error: Unexpected argument '{}'", arg));
                }
            }
            other => return Err(format!("Error: Unknown option '{}'", other)),
        }
        i += 1;
    }

    Ok(PkgCommand::Init { path, name, is_lib })
}

fn parse_pkg_add_args(args: &[String]) -> Result<PkgCommand, String> {
    let mut name = None;
    let mut path = None;
    let mut git = None;
    let mut tag = None;
    let mut branch = None;
    let mut version = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--path" => {
                if i + 1 < args.len() {
                    path = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    return Err("Error: Missing argument for '--path'".to_string());
                }
            }
            "--git" => {
                if i + 1 < args.len() {
                    git = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    return Err("Error: Missing argument for '--git'".to_string());
                }
            }
            "--tag" => {
                if i + 1 < args.len() {
                    tag = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    return Err("Error: Missing argument for '--tag'".to_string());
                }
            }
            "--branch" => {
                if i + 1 < args.len() {
                    branch = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    return Err("Error: Missing argument for '--branch'".to_string());
                }
            }
            "--version" => {
                if i + 1 < args.len() {
                    version = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    return Err("Error: Missing argument for '--version'".to_string());
                }
            }
            arg if !arg.starts_with('-') => {
                if name.is_none() {
                    let (pkg_spec, ver_spec) = if let Some((p, v)) = arg.split_once('@') {
                        (p, Some(v))
                    } else {
                        (arg, None)
                    };
                    if let Some(v) = ver_spec {
                        if version.is_none() {
                            version = Some(v.trim_start_matches('v').to_string());
                        }
                        if tag.is_none() {
                            tag = Some(v.to_string());
                        }
                    }
                    name = Some(pkg_spec.to_string());
                } else {
                    return Err(format!("Error: Unexpected argument '{}'", arg));
                }
            }
            other => return Err(format!("Error: Unknown option '{}'", other)),
        }
        i += 1;
    }

    let name = name.ok_or_else(|| "Error: Missing package name for 'add'".to_string())?;

    Ok(PkgCommand::Add {
        name,
        path,
        git,
        tag,
        branch,
        version,
    })
}

fn parse_pkg_cache_args(args: &[String]) -> Result<PkgCommand, String> {
    let mut clean = false;
    let mut all = false;
    for arg in args {
        match arg.as_str() {
            "clean" | "purge" | "--clean" => clean = true,
            "--all" | "-a" => all = true,
            "-h" | "--help" | "help" => return Ok(PkgCommand::Help),
            other => {
                return Err(format!(
                    "Error: Unknown option '{}' for 'pkg cache'. Run 'alya pkg help' for usage.",
                    other
                ))
            }
        }
    }
    Ok(PkgCommand::Cache { clean, all })
}

fn parse_pkg_clean_args(args: &[String]) -> Result<PkgCommand, String> {
    let mut all = false;
    for arg in args {
        match arg.as_str() {
            "--all" | "-a" => all = true,
            "-h" | "--help" | "help" => return Ok(PkgCommand::Help),
            other => {
                return Err(format!(
                    "Error: Unknown option '{}' for 'pkg clean'. Run 'alya pkg help' for usage.",
                    other
                ))
            }
        }
    }
    Ok(PkgCommand::Clean { all })
}

fn parse_pkg_update_args(args: &[String]) -> Result<PkgCommand, String> {
    let mut upgrade = false;
    for arg in args {
        match arg.as_str() {
            "-u" | "--upgrade" => upgrade = true,
            "-h" | "--help" | "help" => return Ok(PkgCommand::Help),
            other => {
                return Err(format!(
                    "Error: Unknown option '{}' for 'update'. Supported flags: -u, --upgrade",
                    other
                ))
            }
        }
    }
    Ok(PkgCommand::Update { upgrade })
}

fn parse_toolchain_args(args: &[String]) -> Result<ToolchainCommand, String> {
    if args.is_empty() {
        return Ok(ToolchainCommand::Status);
    }
    match args[0].as_str() {
        "status" => Ok(ToolchainCommand::Status),
        "install" => Ok(ToolchainCommand::Install),
        "clean" | "purge" => Ok(ToolchainCommand::Clean),
        "help" | "-h" | "--help" => Ok(ToolchainCommand::Help),
        other => Err(format!(
            "Error: Unknown toolchain subcommand '{}'. Run 'alya toolchain help' for usage.",
            other
        )),
    }
}
