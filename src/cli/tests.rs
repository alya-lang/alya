use super::args::{CliArgs, CommandKind};
use crate::codegen::{Architecture, OperatingSystem};

fn to_args(slice: &[&str]) -> Vec<String> {
    slice.iter().map(|s| s.to_string()).collect()
}

#[test]
fn test_subcommand_run() {
    let args = to_args(&["alya", "run", "hello.alya"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(parsed.command, CommandKind::Run);
    assert_eq!(parsed.input_file, "hello.alya");
    assert!(parsed.output_binary);
}

#[test]
fn test_subcommand_build() {
    let args = to_args(&["alya", "build", "main.alya", "-o", "main.exe"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(parsed.command, CommandKind::Build);
    assert_eq!(parsed.input_file, "main.alya");
    assert_eq!(parsed.output_file, Some("main.exe".into()));
    assert!(parsed.output_binary);
}

#[test]
fn test_subcommand_check() {
    let args = to_args(&["alya", "check", "code.alya"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(parsed.command, CommandKind::Check);
    assert_eq!(parsed.input_file, "code.alya");
}

#[test]
fn test_flag_run_and_quiet() {
    let args = to_args(&["alya", "test.alya", "-r", "-q"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(parsed.command, CommandKind::Run);
    assert!(parsed.output_binary);
    assert!(parsed.quiet);
}

#[test]
fn test_target_arch_and_os() {
    let args = to_args(&["alya", "test.alya", "--arch", "arm64", "--os", "linux"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(parsed.arch, Architecture::ARM64);
    assert_eq!(parsed.os, OperatingSystem::Linux);
}

#[test]
fn test_help_and_version() {
    let args = to_args(&["alya"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(parsed.command, CommandKind::Repl);

    assert_eq!(CliArgs::parse_from(&to_args(&["alya", "--help"])), Ok(None));
    assert_eq!(CliArgs::parse_from(&to_args(&["alya", "help"])), Ok(None));
    assert_eq!(
        CliArgs::parse_from(&to_args(&["alya", "--version"])),
        Ok(None)
    );
    assert_eq!(
        CliArgs::parse_from(&to_args(&["alya", "version"])),
        Ok(None)
    );
}

#[test]
fn test_subcommand_repl() {
    let args = to_args(&["alya", "repl"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(parsed.command, CommandKind::Repl);
}

#[test]
fn test_run_with_double_dash_args() {
    let args = to_args(&[
        "alya",
        "run",
        "spec/syntax/variables.alya",
        "--",
        "foo",
        "bar",
        "baz",
    ]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(parsed.command, CommandKind::Run);
    assert_eq!(parsed.input_file, "spec/syntax/variables.alya");
    assert_eq!(parsed.run_args, vec!["foo", "bar", "baz"]);
}

#[test]
fn test_run_with_trailing_args_without_dash() {
    let args = to_args(&[
        "alya",
        "run",
        "spec/syntax/variables.alya",
        "foo",
        "bar",
        "baz",
    ]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(parsed.command, CommandKind::Run);
    assert_eq!(parsed.input_file, "spec/syntax/variables.alya");
    assert_eq!(parsed.run_args, vec!["foo", "bar", "baz"]);
}

#[test]
fn test_time_and_stats_flags() {
    let args = to_args(&["alya", "build", "main.alya", "--time"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert!(parsed.time);
    assert!(!parsed.stats);

    let args_stats = to_args(&["alya", "main.alya", "--stats"]);
    let parsed_stats = CliArgs::parse_from(&args_stats).unwrap().unwrap();
    assert!(parsed_stats.time);
    assert!(parsed_stats.stats);

    let args_bench = to_args(&["alya", "main.alya", "--bench"]);
    let parsed_bench = CliArgs::parse_from(&args_bench).unwrap().unwrap();
    assert!(parsed_bench.time);
    assert!(parsed_bench.stats);
}

#[test]
fn test_run_with_program_flags_without_double_dash() {
    let args = to_args(&[
        "alya",
        "run",
        "apps/http_server/main.alya",
        "--port",
        "8080",
    ]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(parsed.command, CommandKind::Run);
    assert_eq!(parsed.input_file, "apps/http_server/main.alya");
    assert_eq!(parsed.run_args, vec!["--port", "8080"]);
}

#[test]
fn test_bundle_flags() {
    let args = to_args(&["alya", "build", "game.alya", "--bundle"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert!(parsed.bundle);
    assert!(parsed.output_binary);
    assert_eq!(parsed.os, crate::cli::args::host_operating_system());

    let args_arm = to_args(&["alya", "build", "game.alya", "--bundle", "--arch", "arm64"]);
    let parsed_arm = CliArgs::parse_from(&args_arm).unwrap().unwrap();
    assert!(parsed_arm.bundle);
    assert_eq!(parsed_arm.os, crate::cli::args::host_operating_system());
    assert_eq!(parsed_arm.arch, Architecture::ARM64);

    let args_custom = to_args(&[
        "alya",
        "build",
        "game.alya",
        "--app",
        "--bundle-id",
        "com.mycompany.game",
        "--icon",
        "my_icon.icns",
        "--arch",
        "x64",
    ]);
    let parsed_custom = CliArgs::parse_from(&args_custom).unwrap().unwrap();
    assert!(parsed_custom.bundle);
    assert_eq!(parsed_custom.bundle_id, Some("com.mycompany.game".into()));
    assert_eq!(parsed_custom.icon_path, Some("my_icon.icns".into()));
    assert_eq!(parsed_custom.os, crate::cli::args::host_operating_system());
    assert_eq!(parsed_custom.arch, Architecture::X64);
}

#[test]
fn test_gui_flag_implies_bundle() {
    let args = to_args(&["alya", "build", "game.alya", "--gui"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert!(parsed.gui);
    assert!(parsed.bundle);
    assert!(parsed.output_binary);
    // Bare --gui targets the host platform (runnable where built).
    assert_eq!(parsed.os, crate::cli::args::host_operating_system());

    let args_win = to_args(&["alya", "build", "game.alya", "--gui", "--os", "windows"]);
    let parsed_win = CliArgs::parse_from(&args_win).unwrap().unwrap();
    assert!(parsed_win.gui);
    assert_eq!(parsed_win.os, OperatingSystem::Windows);

    let args_linux = to_args(&["alya", "build", "game.alya", "--bundle", "--os", "linux"]);
    let parsed_linux = CliArgs::parse_from(&args_linux).unwrap().unwrap();
    assert!(parsed_linux.bundle);
    assert!(!parsed_linux.gui);
    assert_eq!(parsed_linux.os, OperatingSystem::Linux);
}

#[test]
fn test_pkg_init_cli() {
    let args = to_args(&["alya", "init", "my_pkg", "--lib"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    match parsed.command {
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Init { path, name, is_lib }) => {
            assert_eq!(path, Some("my_pkg".to_string()));
            assert_eq!(name, None);
            assert!(is_lib);
        }
        _ => panic!("Expected PkgCommand::Init"),
    }
}

#[test]
fn test_pkg_add_cli() {
    let args = to_args(&["alya", "add", "raylib", "--path", "../raylib"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    match parsed.command {
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Add {
            name,
            path,
            git,
            tag,
            ..
        }) => {
            assert_eq!(name, "raylib");
            assert_eq!(path, Some("../raylib".to_string()));
            assert_eq!(git, None);
            assert_eq!(tag, None);
        }
        _ => panic!("Expected PkgCommand::Add"),
    }
}

#[test]
fn test_pkg_add_version_at_syntax() {
    let args = to_args(&["alya", "add", "http@0.1.0"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    match parsed.command {
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Add {
            name, version, tag, ..
        }) => {
            assert_eq!(name, "http");
            assert_eq!(version, Some("0.1.0".to_string()));
            assert_eq!(tag, Some("0.1.0".to_string()));
        }
        _ => panic!("Expected PkgCommand::Add"),
    }
}

#[test]
fn test_pkg_add_shorthand_syntax() {
    let args = to_args(&["alya", "add", "alya-lang/crypto@v0.1.0"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    match parsed.command {
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Add {
            name, version, tag, ..
        }) => {
            assert_eq!(name, "alya-lang/crypto");
            assert_eq!(version, Some("0.1.0".to_string()));
            assert_eq!(tag, Some("v0.1.0".to_string()));
        }
        _ => panic!("Expected PkgCommand::Add"),
    }
}

#[test]
fn test_pkg_install_cli() {
    let args = to_args(&["alya", "install"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(
        parsed.command,
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Install { strict: false })
    );

    let args = to_args(&["alya", "install", "--strict"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(
        parsed.command,
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Install { strict: true })
    );

    let args = to_args(&["alya", "pkg", "install", "--strict"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(
        parsed.command,
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Install { strict: true })
    );
}

#[test]
fn test_pkg_subcommands() {
    let args = to_args(&["alya", "pkg", "list"]);
    let parsed = CliArgs::parse_from(&args).unwrap().unwrap();
    assert_eq!(
        parsed.command,
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::List)
    );

    let args2 = to_args(&["alya", "pkg", "update"]);
    let parsed2 = CliArgs::parse_from(&args2).unwrap().unwrap();
    assert_eq!(
        parsed2.command,
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Update { upgrade: false })
    );

    let args_u = to_args(&["alya", "update", "-u"]);
    let parsed_u = CliArgs::parse_from(&args_u).unwrap().unwrap();
    assert_eq!(
        parsed_u.command,
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Update { upgrade: true })
    );

    let args_outdated = to_args(&["alya", "pkg", "outdated"]);
    let parsed_outdated = CliArgs::parse_from(&args_outdated).unwrap().unwrap();
    assert_eq!(
        parsed_outdated.command,
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Update { upgrade: false })
    );

    let args3 = to_args(&["alya", "pkg", "cache"]);
    let parsed3 = CliArgs::parse_from(&args3).unwrap().unwrap();
    assert_eq!(
        parsed3.command,
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Cache {
            clean: false,
            all: false
        })
    );

    let args4 = to_args(&["alya", "pkg", "cache", "clean", "--all"]);
    let parsed4 = CliArgs::parse_from(&args4).unwrap().unwrap();
    assert_eq!(
        parsed4.command,
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Cache {
            clean: true,
            all: true
        })
    );

    let args5 = to_args(&["alya", "pkg", "clean"]);
    let parsed5 = CliArgs::parse_from(&args5).unwrap().unwrap();
    assert_eq!(
        parsed5.command,
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Clean { all: false })
    );

    let args6 = to_args(&["alya", "cache"]);
    let parsed6 = CliArgs::parse_from(&args6).unwrap().unwrap();
    assert_eq!(
        parsed6.command,
        CommandKind::Pkg(crate::tools::pkg::PkgCommand::Cache {
            clean: false,
            all: false
        })
    );
}

#[test]
fn test_subcommand_test_with_jobs_and_sequential() {
    let args1 = to_args(&["alya", "test", "--sequential"]);
    let parsed1 = CliArgs::parse_from(&args1).unwrap().unwrap();
    assert_eq!(parsed1.command, CommandKind::Test);
    assert_eq!(parsed1.test_jobs, Some(1));

    let args2 = to_args(&["alya", "test", "-j", "4"]);
    let parsed2 = CliArgs::parse_from(&args2).unwrap().unwrap();
    assert_eq!(parsed2.command, CommandKind::Test);
    assert_eq!(parsed2.test_jobs, Some(4));

    let args3 = to_args(&["alya", "test", "--jobs", "8"]);
    let parsed3 = CliArgs::parse_from(&args3).unwrap().unwrap();
    assert_eq!(parsed3.command, CommandKind::Test);
    assert_eq!(parsed3.test_jobs, Some(8));
}

#[test]
fn test_mem_trace_flag() {
    let args1 = to_args(&["alya", "run", "main.alya", "--mem-trace"]);
    let parsed1 = CliArgs::parse_from(&args1).unwrap().unwrap();
    assert_eq!(parsed1.command, CommandKind::Run);
    assert!(parsed1.mem_trace);

    let args2 = to_args(&["alya", "build", "main.alya"]);
    let parsed2 = CliArgs::parse_from(&args2).unwrap().unwrap();
    assert!(!parsed2.mem_trace);
}
