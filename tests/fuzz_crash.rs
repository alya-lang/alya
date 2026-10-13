//! Crash fuzzer (alya-lang/alya#182, P0).
//!
//! Generates random but well-typed Alya programs, compiles and runs
//! each, and requires exit code 0 plus the `done` marker line on
//! stdout. The oracle is deliberately narrow but strict: such crashes
//! (access violations, silent early exits, link failures) and
//! truncated-output regressions (a missing `done` line means control
//! never reached the end) need no per-case expected output to detect,
//! and the #178-#181 class all crashed while producing plausible
//! output or none.
//!
//! Every generated program is valid by construction (typed lets,
//! declared fields, bounded loops, terminating fibers), so any
//! nonzero exit, missing `done` marker, compiler panic, or gcc
//! failure is a compiler bug: the harness panics with the full
//! sources attached for repro.
//!
//! Determinism: xorshift64* with a fixed seed — same seed, same
//! programs, same verdict on every platform. Scale with
//! `ALYA_FUZZ_N` (default 40, ~1-3 min; nightly uses 400+).
//! Per-case watchdog (20 s, std only) keeps one hung binary from
//! wedging CI; bounded loops mean hangs themselves are findings.
//!
//! Boundary mutations mirror the interaction axes behind #178-#181
//! plus float and channel shapes (alya-lang/alya#189): plain,
//! inside-a-function, cross-module (helper file + alias import),
//! loop-wrapped try/return, spawn-capture, float arithmetic and
//! float containers, buffered-channel send/recv round-trips,
//! dynamic (`any`) holes. Extend by adding shapes to `emit_case`,
//! never by weakening the oracle.

mod common;

use alya::codegen::{self, Architecture, OperatingSystem};
use alya::lexer::Lexer;
use alya::parser::Parser;
use common::{execution_skip_reason, harness_gcc};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static FUZZ_ID: AtomicU64 = AtomicU64::new(0);

/// Deterministic xorshift64* (no dependency).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next() % n as u64) as usize
    }

    fn coin(&mut self) -> bool {
        self.next() & 1 == 1
    }

    fn small_int(&mut self) -> i64 {
        // Small and benign: no overflow/div-zero games (0 included for
        // null-trap-adjacent paths, never as a divisor — `/` and `%`
        // are not generated at all).
        match self.below(6) {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 7,
            4 => 42,
            _ => -(self.below(50) as i64),
        }
    }

    fn small_str(&mut self) -> &'static str {
        const POOL: &[&str] = &["a", "label", "hello", "x", "boom", "fallback", ""];
        POOL[self.below(POOL.len())]
    }
}

fn host_os() -> OperatingSystem {
    if cfg!(target_os = "windows") {
        OperatingSystem::Windows
    } else if cfg!(target_os = "macos") {
        OperatingSystem::MacOS
    } else {
        OperatingSystem::Linux
    }
}

fn host_arch() -> Architecture {
    if cfg!(target_arch = "aarch64") {
        Architecture::ARM64
    } else {
        Architecture::X64
    }
}

/// One generated case: entry source plus optional helper files
/// `(filename, source)` for the cross-module shape.
struct Case {
    entry: String,
    helpers: Vec<(String, String)>,
}

fn emit_case(rng: &mut Rng, idx: usize) -> Case {
    let cross_module = idx % 3 == 2;
    let mut helpers = Vec::new();
    // Struct spelling seen by main: bare locally, qualified across.
    let struct_ref = if cross_module { "h::Fz" } else { "Fz" };
    let make_call = |a: i64, b: i64| -> String {
        if cross_module {
            format!("h::make_fz({}, {})", a, b)
        } else {
            format!("Fz {{ a: {}, b: {} }}", a, b)
        }
    };

    let mut prelude = String::new();
    if cross_module {
        // Helper owns the struct, its factory, and overlap-free
        // operator overloads (the #180 shape).
        helpers.push((
            "fz_helper.alya".to_string(),
            "pub struct Fz\n    a: int,\n    b: int\nend\n\npub function Fz.operator+(self: Fz, other: Fz) -> Fz\n    return Fz { a: self.a + other.a, b: self.b + other.b }\nend\n\npub function Fz.operator==(self: Fz, other: Fz) -> bool\n    return self.a == other.a and self.b == other.b\nend\n\npub function make_fz(a: int, b: int) -> Fz\n    return Fz { a: a, b: b }\nend\n".to_string(),
        ));
        prelude.push_str("import \"./fz_helper.alya\" as h\n\n");
    } else {
        prelude.push_str("struct Fz\n    a: int,\n    b: int\nend\n\n");
        prelude.push_str("function Fz.operator+(self: Fz, other: Fz) -> Fz\n    return Fz { a: self.a + other.a, b: self.b + other.b }\nend\n\n");
        prelude.push_str("function Fz.operator==(self: Fz, other: Fz) -> bool\n    return self.a == other.a and self.b == other.b\nend\n\n");
        // Untyped field-reader helper (the #178 shape: catch values
        // and struct args flow through untyped params).
        prelude.push_str(
            "function show_fz(err) -> string\n    return f\"fz: {err.a} / {err.b}\"\nend\n\n",
        );
        prelude.push_str("function show_msg(err) -> string\n    return f\"m: {err.a}\"\nend\n\n");
        // try/return helper for loop stress (the #179 shape).
        prelude.push_str("function guarded_fz(x: string?) -> string\n    try\n        return x ?? \"fallback\"\n    catch err\n        return \"caught\"\n    end\nend\n\n");
        // Float helper (alya-lang/alya#189): exact float arithmetic
        // through a typed boundary (int-bits-as-double guards).
        prelude.push_str("function addf(x: float, y: float) -> float\n    return x + y\nend\n\n");
        // Channel methods link against std/sync (see Chapter 17 and
        // the syntax/concurrency.alya note); the import covers the
        // whole file, same as the golden fixture.
        prelude.insert_str(0, "import \"std/sync\"\n\n");
    }

    let mut body = String::new();
    let a0 = rng.small_int().abs();
    let b0 = rng.small_int().abs();
    let a1 = rng.small_int().abs();
    let b1 = rng.small_int().abs();
    body.push_str(&format!(
        "    let p = {}\n    let q = {}\n",
        make_call(a0, b0),
        make_call(a1, b1)
    ));
    // Overloaded + and == (valid: same-type struct operands).
    body.push_str("    let s = p + q\n");
    body.push_str(&format!("    assert s.a == {}\n", a0 + a1));
    body.push_str(&format!("    assert s.b == {}\n", b0 + b1));
    if a0 == a1 && b0 == b1 {
        body.push_str("    assert p == q\n");
        body.push_str("    assert not (p != q)\n");
    } else {
        body.push_str("    assert not (p == q)\n");
    }
    if rng.coin() {
        // if/else on the same condition (exercises both arms).
        body.push_str("    if p == q\n");
        body.push_str("        assert s.a == s.a\n");
        body.push_str("    else\n");
        body.push_str("        assert s.b == s.b\n");
        body.push_str("    end\n");
    }
    // try/catch with a struct throw; direct reads plus, same-file
    // only, helper flow (the #178 shape needs untyped helpers).
    body.push_str(&format!(
        "    try\n        throw {} {{ a: {}, b: {} }}\n    catch err\n",
        struct_ref,
        rng.small_int().abs(),
        rng.small_int().abs()
    ));
    if cross_module {
        body.push_str("        assert err.a >= 0\n");
    } else {
        body.push_str("        say show_fz(err)\n");
        body.push_str("        say show_msg(err)\n");
    }
    body.push_str("    end\n");
    // try/return loop stress (the #179 shape), same-file only: the
    // helper lives in the prelude.
    if !cross_module {
        let reps = 20 + rng.below(40);
        body.push_str("    let t = 0\n");
        body.push_str(&format!("    while t < {}\n", reps));
        body.push_str("        guarded_fz(\"bench\")\n");
        body.push_str("        t += 1\n");
        body.push_str("    end\n");
        body.push_str("    assert guarded_fz(\"hi\") == \"hi\"\n");
        body.push_str("    assert guarded_fz(null) == \"fallback\"\n");
    }
    // spawn closure capturing a local (the #181 shape). Fibers run at
    // drain; the say proves the cell read without ordering asserts.
    let cap = rng.small_int().abs() + 1;
    body.push_str(&format!("    let tag = {}\n", cap));
    body.push_str("    spawn || =>\n");
    body.push_str("        say f\"fiber sees {tag}\"\n");
    body.push_str("    end\n");
    // Dynamic hole: any-typed roundtrip through a helper-free read.
    if rng.coin() {
        body.push_str("    let dynv: any = s.a\n");
        body.push_str("    assert dynv == s.a\n");
    }
    // Float shapes (alya-lang/alya#189): exact arithmetic through a
    // typed helper plus a float container round-trip. Literals are
    // exactly representable, so equality is deterministic.
    body.push_str("    let fa = 1.5\n");
    body.push_str("    let fb = 2.5\n");
    body.push_str("    assert fa + fb == 4.0\n");
    if !cross_module {
        body.push_str("    assert addf(fa, fb) == 4.0\n");
    }
    body.push_str("    let farr = []\n");
    body.push_str("    push(farr, fa)\n");
    body.push_str("    assert farr[0] == fa\n");
    // Buffered-channel round-trip (alya-lang/alya#189), same-file
    // only: the std/sync import lives in the prelude there.
    if !cross_module {
        body.push_str("    let fzch = Channel[int].new(capacity: 4)\n");
        body.push_str("    fzch.send(11)\n");
        body.push_str("    fzch.send(22)\n");
        body.push_str("    assert fzch.recv() == 11\n");
        body.push_str("    assert fzch.recv() == 22\n");
    }
    // Interpolation with mixed holes.
    body.push_str(&format!(
        "    say f\"done {{s.a}} {{s.b}} {}\"\n",
        rng.small_str()
    ));

    let mut entry = prelude;
    entry.push_str("function main()\n");
    entry.push_str(&body);
    entry.push_str("end\n\nmain()\n");
    Case { entry, helpers }
}

/// Compile + run one case in a fresh temp dir. Returns workdir + log
/// on failure for the panic message.
fn run_case(case: &Case, idx: usize) -> Result<(), String> {
    let pid = std::process::id();
    let id = FUZZ_ID.fetch_add(1, Ordering::SeqCst);
    let dir: PathBuf = std::env::temp_dir().join(format!("alya-fuzz-{}-{}-{}", pid, idx, id));
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {}", e))?;
    for (name, src) in &case.helpers {
        fs::write(dir.join(name), src).map_err(|e| format!("write helper: {}", e))?;
    }
    let main_path = dir.join("main.alya");
    fs::write(&main_path, &case.entry).map_err(|e| format!("write main: {}", e))?;

    let r = std::panic::catch_unwind(|| -> Result<(i32, String), String> {
        let mut lexer = Lexer::new(&case.entry);
        let tokens = lexer.tokenize().map_err(|e| format!("lex: {}", e))?;
        let mut parser = Parser::new(tokens);
        let mut ast = parser.parse().map_err(|e| format!("parse: {}", e))?;
        alya::parser::resolve_imports(&mut ast, &dir, &alya::parser::CfgContext::host())
            .map_err(|e| format!("resolve: {}", e))?;
        let asm_code = codegen::generate(&ast, host_arch(), host_os());

        let asm_path = dir.join("fuzz.s");
        let exe_path = if cfg!(target_os = "windows") {
            dir.join("fuzz.exe")
        } else {
            dir.join("fuzz")
        };
        fs::write(&asm_path, &asm_code).map_err(|e| format!("write asm: {}", e))?;
        let mut gcc = std::process::Command::new(harness_gcc());
        gcc.arg(&asm_path).arg("-o").arg(&exe_path);
        if matches!(host_os(), OperatingSystem::Linux) {
            gcc.arg("-no-pie");
            gcc.arg("-lm");
        }
        if matches!(host_os(), OperatingSystem::Windows) {
            gcc.arg("-lws2_32");
        }
        if matches!(host_os(), OperatingSystem::MacOS) {
            gcc.arg("-Wl,-dead_strip");
        } else {
            gcc.arg("-Wl,--gc-sections");
        }
        let gcc_out = gcc.output().map_err(|e| format!("gcc spawn: {}", e))?;
        if !gcc_out.status.success() {
            return Err(format!(
                "gcc failed:\n{}",
                String::from_utf8_lossy(&gcc_out.stderr)
            ));
        }
        // Watchdog: bounded programs must finish quickly; a hang is a
        // finding, not a wedged suite (std only, no extra deps).
        let done = AtomicBool::new(false);
        let prog_out = std::thread::scope(|s| {
            let handle = s.spawn(|| {
                let out = std::process::Command::new(&exe_path)
                    .output()
                    .map_err(|e| format!("exec: {}", e));
                done.store(true, Ordering::SeqCst);
                out
            });
            for _ in 0..200 {
                if done.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            if !done.load(Ordering::SeqCst) {
                return Err("timeout: 20 s without exit (hang)".to_string());
            }
            handle.join().map_err(|_| "worker panicked".to_string())?
        })?;
        let code = prog_out.status.code().unwrap_or(-1);
        let mut output = String::from_utf8_lossy(&prog_out.stdout).replace("\r\n", "\n");
        output.push_str(&String::from_utf8_lossy(&prog_out.stderr).replace("\r\n", "\n"));
        Ok((code, output))
    });

    // Best-effort cleanup; keep the dir on failure for forensics.
    // Oracle (alya-lang/alya#189): exit 0 AND the `done` marker on
    // stdout. Every generated case prints one, so a missing marker
    // means a silent early exit or truncated output — a finding, not
    // a pass.
    let failed = !matches!(&r, Ok(Ok((0, ref out))) if out.contains("done"));
    if !failed {
        let _ = fs::remove_dir_all(&dir);
        return Ok(());
    }
    let detail = match r {
        Ok(Ok((code, output))) => format!("exit={}\noutput:\n{}", code, output),
        Ok(Err(e)) => e,
        Err(_) => "compiler panicked".to_string(),
    };
    let mut repro = format!("=== fuzz case {} FAILED\n--- dir: {}\n", idx, dir.display());
    for (name, src) in &case.helpers {
        repro.push_str(&format!("--- helper {}\n{}\n", name, src));
    }
    repro.push_str(&format!("--- main.alya\n{}\n--- {}\n", case.entry, detail));
    Err(repro)
}

#[test]
fn test_fuzz_crash_seeded() {
    if let Some(reason) = execution_skip_reason() {
        println!("SKIP fuzz: {}", reason);
        return;
    }
    if std::process::Command::new(harness_gcc())
        .arg("--version")
        .output()
        .is_err()
    {
        println!("SKIP fuzz: no C toolchain in PATH");
        return;
    }
    let n: usize = std::env::var("ALYA_FUZZ_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(40);
    let mut rng = Rng(0x1A1A5EED_20261013);
    let mut failures = Vec::new();
    for i in 0..n {
        let case = emit_case(&mut rng, i);
        if let Err(repro) = run_case(&case, i) {
            failures.push(repro);
            if failures.len() >= 3 {
                break;
            }
        }
        if i % 10 == 9 {
            println!("  [FUZZ] {}/{} cases clean", i + 1, n);
        }
    }
    assert!(
        failures.is_empty(),
        "fuzz found {} crashing case(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
    println!("  [FUZZ] all {} cases exited 0 with the done marker", n);
}
