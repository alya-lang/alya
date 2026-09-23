use crate::ast::{Expr, Program, Stmt};
use crate::codegen::{self, Architecture, OperatingSystem};
use crate::driver::runner;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::tools::tool_config::SuiteConfig;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn is_ignored_test_dir(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "target"
                | "build"
                | "fixtures"
                | "fixture"
                | "testdata"
                | "common"
                | "helpers"
                | "mock"
                | "mocks"
                | "node_modules"
                | "vendor"
                // Spec rejection fixtures: intentionally unparseable, never
                // runnable suites (covered by negative_spec_tests instead).
                | "negative"
        )
}

fn is_test_file(name: &str) -> bool {
    if !name.ends_with(".alya") {
        return false;
    }
    name.starts_with("test_") || name.ends_with("_test.alya") || name.ends_with(".test.alya")
}

fn collect_test_files_recursive(dir: &Path, config: &SuiteConfig, tests: &mut Vec<PathBuf>) {
    collect_suite_files_recursive(dir, SuiteKind::Test, config, tests);
}

/// Unified suite discovery: files matching by NAME (`test_*`, `bench_*`,
/// …) plus files containing suite entries by CONTENT (`test` blocks / `@test`
/// for Test kind, `bench` blocks / `@bench` for Bench kind), so suites placed
/// alongside production code (Chapter 22 §1.3) are found without renaming.
/// Unparseable files are skipped here (name-matched ones still fail loudly
/// at execution); ignored directories (including `negative/`) never entered.
/// Project excludes from `.alyatest` / `alya.toml` (`[test]` or `[bench]`)
/// prune whole subtrees on top of the built-in skips.
fn collect_suite_files_recursive(
    dir: &Path,
    kind: SuiteKind,
    config: &SuiteConfig,
    out: &mut Vec<PathBuf>,
) {
    let is_bench = matches!(kind, SuiteKind::Bench);
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if is_ignored_test_dir(file_name) {
            continue;
        }
        if p.is_dir() {
            if !config.is_path_excluded(&p, is_bench) {
                collect_suite_files_recursive(&p, kind, config, out);
            }
            continue;
        }
        if p.extension().and_then(|e| e.to_str()) != Some("alya") {
            continue;
        }
        if config.is_path_excluded(&p, is_bench) {
            continue;
        }
        let name_matched = match kind {
            SuiteKind::Test => is_test_file(file_name),
            SuiteKind::Bench => is_bench_file(file_name),
        };
        if name_matched || file_contains_suite_entries(&p, kind) {
            out.push(p);
        }
    }
}

/// Parses a file and reports whether it declares suite entries for `kind`.
/// Returns false (skip) when the file cannot even be lexed/parsed.
fn file_contains_suite_entries(path: &Path, kind: SuiteKind) -> bool {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(_) => return false,
    };
    let tokens = match Lexer::new(&source).tokenize() {
        Ok(tokens) => tokens,
        Err(_) => return false,
    };
    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(program) => !discover_suite_entry_points(&program, kind).is_empty(),
        Err(_) => false,
    }
}

/// Test vs benchmark suite kind. `alya test` invokes `test` blocks and
/// `@test` functions; `alya bench` invokes `bench` blocks and `@bench`
/// functions (Chapter 22 §1.3–1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuiteKind {
    Test,
    Bench,
}

impl SuiteKind {
    fn entry_prefix(self) -> &'static str {
        match self {
            SuiteKind::Test => "__test_",
            SuiteKind::Bench => "__bench_",
        }
    }

    fn attr_name(self) -> &'static str {
        match self {
            SuiteKind::Test => "test",
            SuiteKind::Bench => "bench",
        }
    }
}

/// Discovers suite entry points in a parsed (pre-resolve) program, in source
/// order: lowered blocks (`__test_*` / `__bench_*`) and `@test` / `@bench`
/// functions. Only zero-parameter functions qualify — anything needing
/// arguments cannot be auto-invoked.
pub fn discover_suite_entry_points(program: &Program, kind: SuiteKind) -> Vec<String> {
    let mut entries = Vec::new();
    for stmt in &program.statements {
        if let Stmt::Function {
            name,
            params,
            attributes,
            ..
        } = stmt.inner_stmt()
        {
            if !params.is_empty() {
                continue;
            }
            if name.starts_with(kind.entry_prefix())
                || (!name.contains("__") && attributes.iter().any(|a| a.name == kind.attr_name()))
            {
                entries.push(name.clone());
            }
        }
    }
    entries
}

/// Collects every callee name referenced in the program (bare and qualified),
/// so tests that are already invoked manually are not called twice.
fn collect_called_names(program: &Program) -> HashSet<String> {
    fn bare(name: &str) -> String {
        let b = name.rsplit("::").next().unwrap_or(name);
        b.rsplit("__").next().unwrap_or(b).to_string()
    }
    fn walk_expr(expr: &Expr, out: &mut HashSet<String>) {
        match expr {
            Expr::Call { name, args } => {
                out.insert(name.clone());
                out.insert(bare(name));
                for a in args {
                    walk_expr(a, out);
                }
            }
            Expr::OptionalCall { callee, args } => {
                out.insert(callee.clone());
                out.insert(bare(callee));
                for a in args {
                    walk_expr(a, out);
                }
            }
            Expr::Binary { left, right, .. } => {
                walk_expr(left, out);
                walk_expr(right, out);
            }
            Expr::Unary { expr, .. } | Expr::ForceUnwrap(expr) => walk_expr(expr, out),
            Expr::Array(items) | Expr::InterpolatedString(items) => {
                for e in items {
                    walk_expr(e, out);
                }
            }
            Expr::Index { array, index } => {
                walk_expr(array, out);
                walk_expr(index, out);
            }
            Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
                walk_expr(object, out)
            }
            Expr::StructInit { fields, .. } => {
                for (_, v) in fields {
                    walk_expr(v, out);
                }
            }
            Expr::Map(entries) => {
                for (k, v) in entries {
                    walk_expr(k, out);
                    walk_expr(v, out);
                }
            }
            Expr::Ternary {
                condition,
                then_branch,
                else_branch,
            } => {
                walk_expr(condition, out);
                walk_expr(then_branch, out);
                walk_expr(else_branch, out);
            }
            Expr::NullCoalesce { value, default } => {
                walk_expr(value, out);
                walk_expr(default, out);
            }
            Expr::Cast { expr, .. } | Expr::TypeCheck { expr, .. } => walk_expr(expr, out),
            _ => {}
        }
    }
    fn walk_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
        if let Stmt::Function { body, .. } = stmt.inner_stmt() {
            for s in body {
                walk_stmt(s, out);
            }
        }
        // Top-level and nested expressions: walk generically via debug shape.
        // (Function bodies covered above; other statements hold expressions
        // in known positions — cover the common ones.)
        match stmt.inner_stmt() {
            Stmt::Say(e) | Stmt::Expr(e) | Stmt::Return(Some(e)) | Stmt::Throw(Some(e)) => {
                walk_expr(e, out)
            }
            Stmt::Let { value, .. } | Stmt::Const { value, .. } => walk_expr(value, out),
            Stmt::Assign { value, .. } => walk_expr(value, out),
            _ => {}
        }
    }

    let mut out = HashSet::new();
    for stmt in &program.statements {
        walk_stmt(stmt, &mut out);
        // Recurse into bodies missed above (if/while/for/try/defer blocks).
        fn walk_blocks(stmt: &Stmt, out: &mut HashSet<String>) {
            let blocks: Vec<&Vec<Stmt>> = match stmt.inner_stmt() {
                Stmt::Function { body, .. } => vec![body],
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    let mut v = vec![then_block];
                    if let Some(eb) = else_block {
                        v.push(eb);
                    }
                    v
                }
                Stmt::While { body, .. } | Stmt::Repeat { body, .. } => vec![body],
                Stmt::For { body, .. } => vec![body],
                Stmt::ForEach { body, .. } => vec![body],
                Stmt::TryCatch {
                    try_block,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    let mut v = vec![try_block, catch_block];
                    if let Some(fb) = finally_block {
                        v.push(fb);
                    }
                    v
                }
                Stmt::Defer(inner) | Stmt::Pub(inner) => {
                    walk_blocks(inner, out);
                    vec![]
                }
                _ => vec![],
            };
            for b in blocks {
                for s in b {
                    walk_stmt(s, out);
                    walk_blocks(s, out);
                }
            }
        }
        walk_blocks(stmt, &mut out);
    }
    out
}

/// Appends synthesized invocations for pre-discovered test entries that are
/// not already called manually. Entries come from the pre-resolve AST;
/// the called-names check runs on the current (resolved) program.
pub fn synthesize_test_calls(program: &mut Program, entries: &[String]) {
    if entries.is_empty() {
        return;
    }
    let called = collect_called_names(program);
    for name in entries {
        let bare = name.rsplit("::").next().unwrap_or(name.as_str());
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        if called.contains(name.as_str()) || called.contains(bare) {
            continue;
        }
        program.statements.push(Stmt::Expr(Expr::Call {
            name: name.clone(),
            args: Vec::new(),
        }));
    }
}

/// Discovers test files in the specified path: name-matched (`test_*`,
/// `*_test`, `*.test`) plus content-matched (files declaring `test` blocks
/// or `@test` functions anywhere under the path). Project excludes from
/// `.alyatest` / `alya.toml` `[test]` apply to directory walks; an
/// explicitly named single file is always honored.
pub fn discover_test_files(path: &Path) -> Vec<PathBuf> {
    let mut tests = Vec::new();

    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("alya") {
            tests.push(path.to_path_buf());
        }
        return tests;
    }

    let config = SuiteConfig::discover(path);
    collect_test_files_recursive(path, &config, &mut tests);

    tests.sort();
    tests
}

fn is_bench_file(name: &str) -> bool {
    if !name.ends_with(".alya") {
        return false;
    }
    name.starts_with("bench_") || name.ends_with("_bench.alya") || name.ends_with(".bench.alya")
}

fn collect_bench_files_recursive(dir: &Path, config: &SuiteConfig, benches: &mut Vec<PathBuf>) {
    collect_suite_files_recursive(dir, SuiteKind::Bench, config, benches);
}

/// Discovers benchmark files in the specified path: name-matched
/// (`bench_*`, `*_bench`, `*.bench`) plus content-matched (files declaring
/// `bench` blocks or `@bench` functions anywhere under the path). Project
/// excludes from `.alyatest` / `alya.toml` `[bench]` apply to directory
/// walks; an explicitly named single file is always honored.
pub fn discover_bench_files(path: &Path) -> Vec<PathBuf> {
    let mut benches = Vec::new();

    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("alya") {
            benches.push(path.to_path_buf());
        }
        return benches;
    }

    let config = SuiteConfig::discover(path);
    collect_bench_files_recursive(path, &config, &mut benches);

    benches.sort();
    benches
}

#[derive(Debug, Clone)]
pub struct TestExecution {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub elapsed_ms: u128,
    pub exit_status: Option<std::process::ExitStatus>,
    pub is_timeout: bool,
}

fn describe_exit_status(status: &std::process::ExitStatus) -> (bool, Option<i32>, String, bool) {
    let is_success = status.success();
    let code = status.code();

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            let (name, is_crash) = match sig {
                11 => ("SIGSEGV (Segmentation fault - invalid memory access)", true),
                10 => (
                    "SIGBUS (Bus error - unaligned or invalid memory access)",
                    true,
                ),
                5 => ("SIGTRAP (Trace/breakpoint trap)", true),
                6 => ("SIGABRT (Aborted)", true),
                4 => ("SIGILL (Illegal instruction)", true),
                8 => ("SIGFPE (Floating point exception)", true),
                9 => ("SIGKILL (Killed)", false),
                15 => ("SIGTERM (Terminated)", false),
                _ => ("Unknown signal", true),
            };
            return (
                false,
                None,
                format!("Terminated by signal: {} (signal {})", name, sig),
                is_crash,
            );
        }
    }

    if let Some(c) = code {
        let ucode = c as u32;
        let (desc, is_crash) = match ucode {
            0 => ("Success (code 0)".to_string(), false),
            0xC0000005 => (
                "Access Violation (0xC0000005 - Segmentation fault / invalid memory pointer dereference)".to_string(),
                true,
            ),
            0xC00000FD => ("Stack Overflow (0xC00000FD)".to_string(), true),
            0xC000001D => ("Illegal Instruction (0xC000001D)".to_string(), true),
            0xC0000094 => ("Integer Division by Zero (0xC0000094)".to_string(), true),
            0xC0000409 => ("Stack Buffer Overrun / Fast Fail (0xC0000409)".to_string(), true),
            0x80000003 => ("Breakpoint Trap (0x80000003)".to_string(), true),
            0xC000000D => ("Invalid Parameter (0xC000000D)".to_string(), true),
            c if c >= 0x80000000 => (format!("Abnormal OS exception (NTSTATUS: 0x{:08X})", c), true),
            c => (format!("Exited with code {}", c), false),
        };
        (is_success, Some(c), desc, is_crash)
    } else {
        (is_success, None, "Terminated abnormally".to_string(), true)
    }
}

/// Compiles and runs an Alya test file, returning a structured TestExecution.
pub fn execute_test_file(
    path: &Path,
    arch: Architecture,
    os: OperatingSystem,
    kind: SuiteKind,
) -> Result<TestExecution, String> {
    let start_time = Instant::now();
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read file '{}': {}", path.display(), e))?;

    // 1. Lexer
    let mut lexer = Lexer::new(&source);
    let tokens = lexer
        .tokenize()
        .map_err(|e| format!("Lexer error in '{}': {}", path.display(), e))?;

    // 2. Parser
    let mut parser = Parser::new(tokens);
    let mut ast = parser
        .parse()
        .map_err(|e| format!("Parser error in '{}': {}", path.display(), e))?;

    // 3. Module Resolution
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    // Collect suite entry points BEFORE imports merge (foreign `__test_*`
    // / `__bench_*` functions must not be auto-invoked here).
    let test_entries = discover_suite_entry_points(&ast, kind);
    let imported_files = crate::parser::resolve_imports_with_sources(&mut ast, base_dir)
        .map_err(|e| format!("Import resolution error in '{}': {}", path.display(), e))?;

    // 3b. Synthesize invocations for `test` blocks and `@test` functions that
    // are not already called manually (Chapter 18 §1.4, Chapter 22 §1.3).
    synthesize_test_calls(&mut ast, &test_entries);

    // 3c. Function inlining ahead of codegen (Chapter 18 §1.2), so DCE can
    // prune fully-inlined callees.
    crate::parser::inline::inline_functions(&mut ast);

    // 4. Codegen
    let asm_code = codegen::generate(&ast, arch, os);

    // 5. Compile with GCC to temp executable
    let pid = std::process::id();
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir();
    let temp_asm = temp_dir.join(format!("temp_test_{}_{}.s", pid, test_id));
    let temp_exe = if matches!(os, OperatingSystem::Windows) {
        temp_dir.join(format!("temp_test_{}_{}.exe", pid, test_id))
    } else {
        temp_dir.join(format!("temp_test_{}_{}", pid, test_id))
    };

    fs::write(&temp_asm, &asm_code)
        .map_err(|e| format!("Failed to write temporary assembly: {}", e))?;

    let c_plan = crate::driver::c_builder::discover_c_build_plan(path, &imported_files)?;
    let c_objects = crate::driver::c_builder::build_c_objects(&c_plan, arch, os)?;
    let mut extra_libs = codegen::collect_extern_libraries(&ast);
    extra_libs.retain(|lib| !c_plan.provided_libs.contains(lib));

    let asm_str = temp_asm.to_string_lossy().to_string();
    let exe_str = temp_exe.to_string_lossy().to_string();
    let gcc_res =
        runner::compile_with_gcc(&asm_str, &exe_str, arch, os, &extra_libs, &c_objects, &[]);
    let _ = fs::remove_file(&temp_asm);
    if let Err(err) = gcc_res {
        return Err(format!(
            "GCC compilation failed for '{}': {}",
            path.display(),
            err
        ));
    }

    // 6. Execute binary with timeout
    let mut child = Command::new(&temp_exe)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to execute test binary '{}': {}", exe_str, e))?;

    let start_wait = Instant::now();
    let timeout_limit = std::time::Duration::from_secs(60);
    let mut exited = false;
    let mut exit_status = None;

    while start_wait.elapsed() < timeout_limit {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("Failed to check status of '{}': {}", exe_str, e))?
        {
            exit_status = Some(status);
            exited = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let (stdout_bytes, stderr_bytes, is_timeout, final_status) = if exited {
        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to read output of '{}': {}", exe_str, e))?;
        (output.stdout, output.stderr, false, Some(output.status))
    } else {
        let _ = child.kill();
        let output = child
            .wait_with_output()
            .unwrap_or_else(|_| std::process::Output {
                status: exit_status.unwrap_or_default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        (output.stdout, output.stderr, true, Some(output.status))
    };

    let _ = fs::remove_file(&temp_exe);
    let elapsed = start_time.elapsed().as_millis();

    let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
    let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();

    let is_clean_exit = final_status.as_ref().map(|s| s.success()).unwrap_or(false);
    let has_fail = stdout.contains("[FAIL]") || stderr.contains("[FAIL]");
    let has_runtime_err = stdout.contains("Runtime error:") || stderr.contains("Runtime error:");
    let success = is_clean_exit && !has_fail && !has_runtime_err && !is_timeout;

    Ok(TestExecution {
        success,
        stdout,
        stderr,
        elapsed_ms: elapsed,
        exit_status: final_status,
        is_timeout,
    })
}

fn format_test_path(path: &Path, root: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let mut s = rel.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = s.strip_prefix("./") {
        s = stripped.to_string();
    }
    if s.is_empty() {
        if let Some(name) = path.file_name() {
            return name.to_string_lossy().to_string();
        }
    }
    s
}

struct TestResultItem {
    display_name: String,
    outcome: Result<TestExecution, String>,
}

#[derive(Default)]
struct TestStats {
    passed: usize,
    failed: usize,
    assert_passes: usize,
    assert_fails: usize,
    failed_suites: Vec<(String, String)>,
}

fn handle_test_result(
    display_name: &str,
    outcome: Result<TestExecution, String>,
    name_width: usize,
    stats: &mut TestStats,
) {
    match outcome {
        Ok(exec) if exec.success => {
            stats.passed += 1;
            let p_count = exec.stdout.matches("[PASS]").count();
            let f_count = exec.stdout.matches("[FAIL]").count();
            stats.assert_passes += p_count;
            stats.assert_fails += f_count;

            if p_count > 0 {
                println!(
                    "  \x1b[1;32m✓\x1b[0m  {:<width$}  \x1b[90m({} asserts | {:>4} ms)\x1b[0m",
                    display_name,
                    p_count,
                    exec.elapsed_ms,
                    width = name_width
                );
            } else {
                println!(
                    "  \x1b[1;32m✓\x1b[0m  {:<width$}  \x1b[90m({:>4} ms)\x1b[0m",
                    display_name,
                    exec.elapsed_ms,
                    width = name_width
                );
            }
        }
        Ok(exec) => {
            stats.failed += 1;
            let p_count = exec.stdout.matches("[PASS]").count();
            let f_count = exec.stdout.matches("[FAIL]").count();
            stats.assert_passes += p_count;
            stats.assert_fails += f_count;

            let (is_clean_exit, _code, exit_desc, is_crash) = match exec.exit_status {
                Some(ref st) => describe_exit_status(st),
                None => (false, None, "Unknown exit status".to_string(), true),
            };

            // Extract all failed assertion lines from stdout and stderr
            let failed_assertions: Vec<&str> = exec
                .stdout
                .lines()
                .chain(exec.stderr.lines())
                .filter(|l| l.contains("[FAIL]") || l.contains("Runtime error:"))
                .map(|s| s.trim())
                .collect();

            // Find the last test that passed before failure or crash
            let last_pass: Option<String> = exec.stdout.lines().rev().find_map(|line| {
                line.find("[PASS]")
                    .map(|idx| line[idx..].trim().to_string())
            });

            let status_label = if exec.is_timeout {
                "TIMEOUT (60s limit exceeded)".to_string()
            } else if is_crash {
                format!("CRASHED ({})", exit_desc)
            } else if !failed_assertions.is_empty() {
                format!(
                    "FAILED ({} assertion failure{})",
                    failed_assertions.len(),
                    if failed_assertions.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                )
            } else if !is_clean_exit {
                format!("FAILED ({})", exit_desc)
            } else {
                "FAILED".to_string()
            };

            println!(
                "  \x1b[1;31m✗\x1b[0m  {:<width$}  \x1b[90m({:>4} ms)\x1b[0m - \x1b[1;31m{}\x1b[0m",
                display_name,
                exec.elapsed_ms,
                status_label,
                width = name_width
            );

            // Record summary detail for the bottom list
            let detail = if exec.is_timeout {
                "Timed out after 60 seconds".to_string()
            } else if is_crash {
                if let Some(ref lp) = last_pass {
                    format!("{} (crashed after '{}')", exit_desc, lp)
                } else {
                    exit_desc.clone()
                }
            } else if !failed_assertions.is_empty() {
                format!("{} assertion failure(s)", failed_assertions.len())
            } else {
                exit_desc.clone()
            };
            stats.failed_suites.push((display_name.to_string(), detail));

            println!(
                "    \x1b[90m┌────────────────────────────────────────────────────────────\x1b[0m"
            );

            // Section 1: Explicit Failed Assertions Callout
            if !failed_assertions.is_empty() {
                println!(
                    "    \x1b[90m│\x1b[0m \x1b[1;31m✗ Failed Assertions ({} total):\x1b[0m",
                    failed_assertions.len()
                );
                for fail_line in failed_assertions.iter().take(10) {
                    println!("    \x1b[90m│\x1b[0m   \x1b[1;31m• {}\x1b[0m", fail_line);
                }
                if failed_assertions.len() > 10 {
                    println!(
                        "    \x1b[90m│\x1b[0m   \x1b[90m... and {} more failed assertion(s)\x1b[0m",
                        failed_assertions.len() - 10
                    );
                }
                println!("    \x1b[90m├────────────────────────────────────────────────────────────\x1b[0m");
            }

            // Section 2: Premature Termination / Crash Callout
            if is_crash || (!is_clean_exit && failed_assertions.is_empty()) {
                println!("    \x1b[90m│\x1b[0m \x1b[1;31m💥 Process terminated abnormally before suite completed!\x1b[0m");
                println!(
                    "    \x1b[90m│\x1b[0m   \x1b[1mReason\x1b[0m    : \x1b[91m{}\x1b[0m",
                    exit_desc
                );
                if let Some(ref lp) = last_pass {
                    println!(
                        "    \x1b[90m│\x1b[0m   \x1b[1mLast Test\x1b[0m : \x1b[32m{}\x1b[0m",
                        lp
                    );
                    println!("    \x1b[90m│\x1b[0m   \x1b[1mLocation\x1b[0m  : \x1b[93mCrashed immediately after this test.\x1b[0m");
                }
                if p_count > 0 {
                    println!(
                        "    \x1b[90m│\x1b[0m   \x1b[1mProgress\x1b[0m  : {} assertion(s) passed before termination.",
                        p_count
                    );
                }
                println!("    \x1b[90m├────────────────────────────────────────────────────────────\x1b[0m");
            }

            // Section 3: Standard Error (stderr)
            if !exec.stderr.trim().is_empty() {
                println!("    \x1b[90m│\x1b[0m \x1b[1;33mStandard Error (stderr):\x1b[0m");
                for line in exec.stderr.lines().take(25) {
                    println!("    \x1b[90m│\x1b[0m   \x1b[91m{}\x1b[0m", line);
                }
                println!("    \x1b[90m├────────────────────────────────────────────────────────────\x1b[0m");
            }

            // Section 4: Formatted Test Log
            let stdout_trimmed = exec.stdout.trim();
            if stdout_trimmed.is_empty() {
                if exec.stderr.trim().is_empty() {
                    println!(
                        "    \x1b[90m│\x1b[0m \x1b[90m(Process produced no standard output)\x1b[0m"
                    );
                }
            } else {
                let lines: Vec<&str> = exec.stdout.lines().collect();
                if lines.len() <= 60 {
                    for line in lines {
                        if line.contains("[FAIL]") || line.contains("Runtime error:") {
                            println!("    \x1b[90m│\x1b[0m \x1b[1;31m{}\x1b[0m", line);
                        } else if line.contains("[PASS]") {
                            println!("    \x1b[90m│\x1b[0m \x1b[32m{}\x1b[0m", line);
                        } else {
                            println!("    \x1b[90m│\x1b[0m {}", line);
                        }
                    }
                } else {
                    for line in &lines[..15] {
                        if line.contains("[FAIL]") || line.contains("Runtime error:") {
                            println!("    \x1b[90m│\x1b[0m \x1b[1;31m{}\x1b[0m", line);
                        } else if line.contains("[PASS]") {
                            println!("    \x1b[90m│\x1b[0m \x1b[32m{}\x1b[0m", line);
                        } else {
                            println!("    \x1b[90m│\x1b[0m {}", line);
                        }
                    }
                    let omitted = lines.len() - 45;
                    println!("    \x1b[90m│   ... [{} lines omitted] ...\x1b[0m", omitted);
                    for line in &lines[lines.len() - 30..] {
                        if line.contains("[FAIL]") || line.contains("Runtime error:") {
                            println!("    \x1b[90m│\x1b[0m \x1b[1;31m{}\x1b[0m", line);
                        } else if line.contains("[PASS]") {
                            println!("    \x1b[90m│\x1b[0m \x1b[32m{}\x1b[0m", line);
                        } else {
                            println!("    \x1b[90m│\x1b[0m {}", line);
                        }
                    }
                }
            }

            println!(
                "    \x1b[90m└────────────────────────────────────────────────────────────\x1b[0m"
            );
        }
        Err(err) => {
            stats.failed += 1;
            stats.failed_suites.push((
                display_name.to_string(),
                "Compilation / Parser Error".to_string(),
            ));
            println!(
                "  \x1b[1;31m✗\x1b[0m  {:<width$} - \x1b[1;31mERROR\x1b[0m",
                display_name,
                width = name_width
            );
            println!(
                "    \x1b[90m┌────────────────────────────────────────────────────────────\x1b[0m"
            );
            println!("    \x1b[90m│\x1b[0m \x1b[91m{}\x1b[0m", err);
            println!(
                "    \x1b[90m└────────────────────────────────────────────────────────────\x1b[0m"
            );
        }
    }
}

/// Runs the Alya test suite on the specified path with optional parallel worker jobs.
pub fn run_tests(
    path_str: &str,
    arch: Architecture,
    os: OperatingSystem,
    jobs: Option<usize>,
) -> Result<(), String> {
    let root = Path::new(path_str);
    let test_files = discover_test_files(root);

    if test_files.is_empty() {
        println!("No test files found in '{}'.", path_str);
        return Ok(());
    }

    let default_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let max_default = if cfg!(target_os = "windows") { 4 } else { 8 };
    let num_workers = jobs
        .unwrap_or_else(|| default_threads.min(max_default))
        .max(1)
        .min(test_files.len());

    let arch_str = match arch {
        Architecture::X64 => "x64",
        Architecture::ARM64 => "arm64",
        Architecture::X86 => "x86",
    };
    let os_str = match os {
        OperatingSystem::Windows => "windows",
        OperatingSystem::Linux => "linux",
        OperatingSystem::MacOS => "macos",
    };
    let alya_ver = env!("CARGO_PKG_VERSION");
    let mode_str = if num_workers == 1 {
        "sequential".to_string()
    } else {
        format!("parallel, {} workers", num_workers)
    };

    let root_arc = Arc::new(root.to_path_buf());
    let name_width = test_files
        .iter()
        .map(|p| format_test_path(p, root).len())
        .max()
        .unwrap_or(30)
        .max(32);

    println!("\n=== Alya Test Suite v{} ===", alya_ver);
    println!(
        "Target : {}-{} | Concurrency: {}",
        arch_str, os_str, mode_str
    );
    println!(
        "Discovered {} test suite(s) in '{}'\n",
        test_files.len(),
        path_str
    );

    let mut stats = TestStats::default();
    let total_start = Instant::now();

    if num_workers == 1 {
        for file in &test_files {
            let display_name = format_test_path(file, root);
            let outcome = execute_test_file(file, arch, os, SuiteKind::Test);
            handle_test_result(&display_name, outcome, name_width, &mut stats);
        }
    } else {
        let (tx, rx) = mpsc::channel();
        let queue = Arc::new(Mutex::new(test_files.into_iter().collect::<VecDeque<_>>()));

        let mut handles = Vec::new();
        for _ in 0..num_workers {
            let q = Arc::clone(&queue);
            let r_root = Arc::clone(&root_arc);
            let sender = tx.clone();
            handles.push(thread::spawn(move || loop {
                let file = {
                    let mut locked = q.lock().unwrap();
                    locked.pop_front()
                };
                match file {
                    Some(path) => {
                        let display_name = format_test_path(&path, &r_root);
                        let outcome = execute_test_file(&path, arch, os, SuiteKind::Test);
                        let _ = sender.send(TestResultItem {
                            display_name,
                            outcome,
                        });
                    }
                    None => break,
                }
            }));
        }
        drop(tx);

        while let Ok(item) = rx.recv() {
            handle_test_result(&item.display_name, item.outcome, name_width, &mut stats);
        }

        for h in handles {
            let _ = h.join();
        }
    }

    let total_time = total_start.elapsed().as_millis();
    println!("\n------------------------------------------------------------------------");
    if stats.failed == 0 {
        println!(
            "  \x1b[1;32m✓ Test Suites : {} passed, {} total\x1b[0m",
            stats.passed,
            stats.passed + stats.failed
        );
        if stats.assert_passes > 0 {
            println!("    Assertions  : {} passed, 0 failed", stats.assert_passes);
        }
        println!("    Duration    : {} ms ({})", total_time, mode_str);
        println!("    Status      : \x1b[1;32mPASSED\x1b[0m");
        println!("------------------------------------------------------------------------\n");
        Ok(())
    } else {
        println!(
            "  \x1b[1;31m✗ Test Suites : {} passed, {} failed, {} total\x1b[0m",
            stats.passed,
            stats.failed,
            stats.passed + stats.failed
        );
        if stats.assert_fails == 0 && stats.failed > 0 {
            println!(
                "    Assertions  : {} passed, 0 failed \x1b[91m({} suite(s) aborted/crashed prematurely)\x1b[0m",
                stats.assert_passes, stats.failed
            );
        } else if stats.assert_passes > 0 || stats.assert_fails > 0 {
            println!(
                "    Assertions  : {} passed, {} failed",
                stats.assert_passes, stats.assert_fails
            );
        }
        println!("    Duration    : {} ms ({})", total_time, mode_str);
        println!("    Status      : \x1b[1;31mFAILED\x1b[0m");

        if !stats.failed_suites.is_empty() {
            println!("\n  \x1b[1;31mFailed Suites Summary:\x1b[0m");
            for (suite_name, reason) in &stats.failed_suites {
                println!(
                    "    \x1b[1;31m✗\x1b[0m \x1b[1m{}\x1b[0m: {}",
                    suite_name, reason
                );
            }
        }

        println!("------------------------------------------------------------------------\n");
        Err(format!(
            "Test suite completed with {} failure(s).",
            stats.failed
        ))
    }
}

/// Discovers and runs benchmark suites: `bench` blocks and `@bench`
/// functions are auto-invoked (Chapter 22 §1.4). Benchmarks always run
/// sequentially — parallel workers would distort timing.
pub fn run_benches(path_str: &str, arch: Architecture, os: OperatingSystem) -> Result<(), String> {
    let root = Path::new(path_str);
    let bench_files = discover_bench_files(root);

    if bench_files.is_empty() {
        println!("No benchmark files found in '{}'.", path_str);
        return Ok(());
    }

    let arch_str = match arch {
        Architecture::X64 => "x64",
        Architecture::ARM64 => "arm64",
        Architecture::X86 => "x86",
    };
    let os_str = match os {
        OperatingSystem::Windows => "windows",
        OperatingSystem::Linux => "linux",
        OperatingSystem::MacOS => "macos",
    };
    let alya_ver = env!("CARGO_PKG_VERSION");

    let name_width = bench_files
        .iter()
        .map(|p| format_test_path(p, root).len())
        .max()
        .unwrap_or(30)
        .max(32);

    println!("\n=== Alya Benchmark Suite v{} ===", alya_ver);
    println!("Target : {}-{} | Concurrency: sequential", arch_str, os_str);
    println!(
        "Discovered {} benchmark suite(s) in '{}'\n",
        bench_files.len(),
        path_str
    );

    let mut stats = TestStats::default();
    let total_start = Instant::now();

    for file in &bench_files {
        let display_name = format_test_path(file, root);
        let outcome = execute_test_file(file, arch, os, SuiteKind::Bench);
        handle_test_result(&display_name, outcome, name_width, &mut stats);
    }

    let total_time = total_start.elapsed().as_millis();
    println!("\n------------------------------------------------------------------------");
    if stats.failed == 0 {
        println!(
            "  \x1b[1;32m✓ Benchmarks : {} passed, {} total\x1b[0m",
            stats.passed,
            stats.passed + stats.failed
        );
        println!("    Duration    : {} ms (sequential)", total_time);
        println!("    Status      : \x1b[1;32mPASSED\x1b[0m");
        println!("------------------------------------------------------------------------\n");
        Ok(())
    } else {
        println!(
            "  \x1b[1;31m✗ Benchmarks : {} passed, {} failed, {} total\x1b[0m",
            stats.passed,
            stats.failed,
            stats.passed + stats.failed
        );
        println!("------------------------------------------------------------------------\n");
        Err(format!(
            "Benchmark suite completed with {} failure(s).",
            stats.failed
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ignored_test_dir() {
        assert!(is_ignored_test_dir(".git"));
        assert!(is_ignored_test_dir("fixtures"));
        assert!(is_ignored_test_dir("testdata"));
        assert!(is_ignored_test_dir("common"));
        assert!(is_ignored_test_dir("target"));
        assert!(!is_ignored_test_dir("subfolder"));
        assert!(!is_ignored_test_dir("integration"));
    }

    #[test]
    fn test_is_test_file() {
        assert!(is_test_file("test_suite.alya"));
        assert!(is_test_file("app_test.alya"));
        assert!(is_test_file("feature.test.alya"));
        assert!(!is_test_file("module1.alya"));
        assert!(!is_test_file("helper.alya"));
        assert!(!is_test_file("test_suite.rs"));
    }

    #[test]
    fn test_is_bench_file() {
        assert!(is_bench_file("bench_basic.alya"));
        assert!(is_bench_file("crypto_bench.alya"));
        assert!(is_bench_file("matrix.bench.alya"));
        assert!(!is_bench_file("module1.alya"));
        assert!(!is_bench_file("test_basic.alya"));
        assert!(!is_bench_file("bench_basic.rs"));
    }

    #[test]
    fn test_discover_suite_entries() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        let source = "@test\nfunction at_fn()\nend\n\ntest \"blk\"\n    say 1\nend\n\n@bench\nfunction at_bench()\nend\n\nbench \"blk2\"\n    say 2\nend\n";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let tests = discover_suite_entry_points(&program, SuiteKind::Test);
        assert_eq!(tests.len(), 2);
        assert!(tests.contains(&"at_fn".to_string()));
        assert!(tests.iter().any(|n| n.starts_with("__test_")));
        let benches = discover_suite_entry_points(&program, SuiteKind::Bench);
        assert_eq!(benches.len(), 2);
        assert!(benches.contains(&"at_bench".to_string()));
        assert!(benches.iter().any(|n| n.starts_with("__bench_")));
    }

    fn unique_suite_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "alya_suite_cfg_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_discover_respects_suite_config_excludes() {
        let dir = unique_suite_dir("excl");
        fs::write(
            dir.join(".alyatest"),
            "[test]\nexclude = [\"slow\"]\n[bench]\nexclude = [\"heavy\"]\n",
        )
        .unwrap();
        fs::create_dir_all(dir.join("slow")).unwrap();
        fs::create_dir_all(dir.join("heavy")).unwrap();
        // Content-matched suites (no name prefix needed).
        fs::write(
            dir.join("fast.alya"),
            "test \"t\"\n    assert_eq(1, 1)\nend\n",
        )
        .unwrap();
        fs::write(
            dir.join("slow").join("slow.alya"),
            "test \"t\"\n    assert_eq(1, 1)\nend\n",
        )
        .unwrap();
        fs::write(
            dir.join("quick_bench.alya"),
            "@bench\nfunction b_q()\nend\n",
        )
        .unwrap();
        fs::write(
            dir.join("heavy").join("heavy_bench.alya"),
            "@bench\nfunction b_h()\nend\n",
        )
        .unwrap();

        let tests = discover_test_files(&dir);
        assert!(tests.iter().any(|p| p.ends_with("fast.alya")));
        assert!(!tests.iter().any(|p| p.ends_with("slow.alya")));

        let benches = discover_bench_files(&dir);
        assert!(benches.iter().any(|p| p.ends_with("quick_bench.alya")));
        assert!(!benches.iter().any(|p| p.ends_with("heavy_bench.alya")));

        let _ = fs::remove_dir_all(&dir);
    }
}
