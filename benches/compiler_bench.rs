use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use alya::codegen::analysis::ProgramInference;
use alya::codegen::{generate, Architecture, OperatingSystem};
use alya::lexer::Lexer;
use alya::parser::Parser;

// ============================================================================
// Memory Tracking Allocator (Tracking total allocated bytes & alloc counts)
// ============================================================================

struct TrackingAllocator;

static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size > layout.size() {
            ALLOCATED_BYTES.fetch_add(new_size - layout.size(), Ordering::Relaxed);
        }
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn reset_alloc() {
    ALLOCATED_BYTES.store(0, Ordering::SeqCst);
    ALLOC_COUNT.store(0, Ordering::SeqCst);
}

fn get_alloc() -> (usize, usize) {
    (
        ALLOCATED_BYTES.load(Ordering::SeqCst),
        ALLOC_COUNT.load(Ordering::SeqCst),
    )
}

// ============================================================================
// Benchmark Statistics & Formatting
// ============================================================================

pub struct BenchStat {
    pub name: &'static str,
    pub iterations: usize,
    pub mean: Duration,
    pub error: Duration,
    pub std_dev: Duration,
    pub min: Duration,
    pub max: Duration,
    pub allocated_bytes: usize,
    pub alloc_count: usize,
    pub throughput: String,
}

/// Two-sided Student's t-value for 99.9% confidence interval based on degrees of freedom (N - 1)
fn student_t_999(n: usize) -> f64 {
    match n {
        1..=2 => 31.599,
        3 => 12.924,
        4 => 8.610,
        5 => 6.869,
        6 => 5.959,
        7 => 5.408,
        8 => 5.041,
        9 => 4.781,
        10 => 4.587,
        11..=15 => 4.140,
        16..=20 => 3.883,
        21..=30 => 3.646,
        31..=60 => 3.460,
        _ => 3.291,
    }
}

pub fn format_duration(dur: Duration) -> String {
    let nanos = dur.as_nanos();
    if nanos < 1_000 {
        format!("{:.2} ns", nanos as f64)
    } else if nanos < 1_000_000 {
        format!("{:.2} µs", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.2} ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.2} s", dur.as_secs_f64())
    }
}

pub fn format_bytes(bytes: usize) -> String {
    if bytes == 0 {
        "-".to_string()
    } else if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn format_thousands(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    let rem = s.len() % 3;
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (i == rem || (i > rem && (i - rem) % 3 == 0)) {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

fn run_bench_with_setup<S, I, F, R>(
    name: &'static str,
    target_duration: Duration,
    mut setup: S,
    mut f: F,
    throughput_calc: impl Fn(usize, Duration) -> String,
) -> BenchStat
where
    S: FnMut() -> I,
    F: FnMut(I) -> R,
{
    // Warmup
    let warmup_end = Instant::now() + Duration::from_millis(60);
    while Instant::now() < warmup_end {
        let input = setup();
        black_box(f(input));
    }

    // Timed measurements
    let mut times = Vec::new();
    let mut total_allocated_bytes = 0usize;
    let mut total_alloc_count = 0usize;
    let start_all = Instant::now();
    let mut total_iters = 0;

    while start_all.elapsed() < target_duration || total_iters < 10 {
        let input = setup();
        reset_alloc();
        let t0 = Instant::now();
        black_box(f(input));
        let el = t0.elapsed();
        let (bytes, count) = get_alloc();
        times.push(el);
        total_allocated_bytes += bytes;
        total_alloc_count += count;
        total_iters += 1;
    }

    let n = times.len();
    let total_nanos: u128 = times.iter().map(|t| t.as_nanos()).sum();
    let mean_nanos = total_nanos as f64 / n as f64;
    let mean = Duration::from_nanos(mean_nanos as u64);

    let variance = if n > 1 {
        let sum_sq_diff: f64 = times
            .iter()
            .map(|t| {
                let diff = t.as_nanos() as f64 - mean_nanos;
                diff * diff
            })
            .sum();
        sum_sq_diff / (n - 1) as f64
    } else {
        0.0
    };
    let std_dev_nanos = variance.sqrt();
    let std_dev = Duration::from_nanos(std_dev_nanos as u64);

    let t_val = student_t_999(n);
    let error_nanos = t_val * (std_dev_nanos / (n as f64).sqrt());
    let error = Duration::from_nanos(error_nanos as u64);

    let min = *times.iter().min().unwrap();
    let max = *times.iter().max().unwrap();
    let total_duration: Duration = times.iter().copied().sum();
    let throughput = throughput_calc(n, total_duration);

    let allocated_per_op = total_allocated_bytes / n;
    let alloc_count_per_op = total_alloc_count / n;

    BenchStat {
        name,
        iterations: n,
        mean,
        error,
        std_dev,
        min,
        max,
        allocated_bytes: allocated_per_op,
        alloc_count: alloc_count_per_op,
        throughput,
    }
}

fn run_bench<F, R>(
    name: &'static str,
    target_duration: Duration,
    mut f: F,
    throughput_calc: impl Fn(usize, Duration) -> String,
) -> BenchStat
where
    F: FnMut() -> R,
{
    run_bench_with_setup(name, target_duration, || (), |_| f(), throughput_calc)
}

fn sample_large_source() -> String {
    let mut src = String::new();
    src.push_str("# Modern Alya Synthetic Workload for Compiler Benchmarking (Spec v1.0)\n\n");

    // 1. Enums
    src.push_str("enum JobState\n    Queued\n    Running\n    Success\n    Failed\nend\n\n");
    src.push_str("enum Severity\n    Debug\n    Info\n    Warn\n    Error\nend\n\n");

    // 2. Structs with typed fields
    src.push_str("struct Vector3\n    x\n    y\n    z\nend\n\n");
    src.push_str("struct Matrix2x2\n    m00\n    m01\n    m10\n    m11\nend\n\n");
    src.push_str("struct TaskRecord\n    id\n    title\n    priority\n    state\nend\n\n");

    // 3. Interfaces
    src.push_str("interface Evaluator\n    function evaluate(self, input)\nend\n\n");

    // 4. Multiple functions with arithmetic, while loops, and pattern matching
    for i in 0..40 {
        src.push_str(&format!(
            "function compute_kernel_{i}(a, b, c)\n    let base = a * 2 + b * 3 - c / 4\n    let acc = 0\n    let step = 0\n    while step < 30\n        acc = acc + step * base\n        if acc > 500\n            acc = acc % 499\n        else\n            acc = acc + 1\n        end\n        step = step + 1\n    end\n    let modifier = when acc % 4\n        is 0 => 10\n        is 1 => 20\n        is 2 => 30\n        else => 5\n    end\n    return acc * modifier + {i}\nend\n\nfunction transform_vector_{i}(v)\n    let scale = {i}\n    let rx = v.x * scale + v.y\n    let ry = v.y * scale - v.z\n    let rz = v.z * scale + v.x\n    return rx + ry + rz\nend\n\n"
        ));
    }

    // 5. Entry point utilizing collections, string interpolation, and dispatch
    src.push_str(
        "function benchmark_entry()\n    let sum = 0\n    let idx = 0\n    let values = [10, 20, 30, 40, 50]\n    while idx < 40\n        sum = sum + compute_kernel_0(idx, idx + 1, idx + 2)\n        idx = idx + 1\n    end\n    let vec = Vector3(1.5, 2.5, 3.5)\n    let v_res = transform_vector_0(vec)\n    say f\"Benchmark finished: sum={sum}, v_res={v_res}\"\n    return sum\nend\n\nbenchmark_entry()\n",
    );

    src
}

fn main() {
    let os_name = match std::env::consts::OS {
        "windows" => "Windows",
        "linux" => "Linux",
        "macos" => "macOS",
        other => other,
    };
    let arch_name = std::env::consts::ARCH;
    let logical_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let alya_ver = env!("CARGO_PKG_VERSION");

    let host_os = match std::env::consts::OS {
        "windows" => OperatingSystem::Windows,
        "linux" => OperatingSystem::Linux,
        "macos" => OperatingSystem::MacOS,
        _ => OperatingSystem::Windows,
    };
    let host_arch = match std::env::consts::ARCH {
        "x86_64" => Architecture::X64,
        "aarch64" => Architecture::ARM64,
        "x86" => Architecture::X86,
        _ => Architecture::X64,
    };

    let cpu_desc = if let Ok(proc_id) = std::env::var("PROCESSOR_IDENTIFIER") {
        proc_id.trim().to_string()
    } else if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        content
            .lines()
            .find(|line| line.starts_with("model name"))
            .and_then(|line| line.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| format!("{} logical cores", logical_cores))
    } else {
        format!("{} logical cores", logical_cores)
    };

    let source = sample_large_source();
    let source_bytes = source.len();
    let source_lines = source.lines().count();

    println!("// * Summary *\n");
    println!("Alya Compiler Benchmark v{alya_ver}, {os_name} ({arch_name})");
    println!("Processor: {}, {} logical cores", cpu_desc, logical_cores);
    println!("Toolchain: rustc 1.75+ (stable), Profile: Release (opt-level=3, LTO=true)");
    println!(
        "Workload : {} lines, {:.2} KB modern synthetic program (40+ functions, structs, enums, when, collections)\n",
        source_lines,
        source_bytes as f64 / 1024.0
    );

    println!("IterationCount=10+  WarmupDuration=60ms  TargetDuration=400ms\n");

    // 1. Lexer Benchmark
    let lex_stat = run_bench(
        "Lexer::tokenize",
        Duration::from_millis(400),
        || {
            let mut lexer = Lexer::new(&source);
            lexer.tokenize().unwrap()
        },
        |iters, dur| {
            let total_bytes = (source_bytes * iters) as f64;
            let secs = dur.as_secs_f64();
            let mb_per_sec = (total_bytes / (1024.0 * 1024.0)) / secs;
            format!("{:.1} MB/s", mb_per_sec)
        },
    );

    // Pre-tokenize for Parser
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize().unwrap();
    let token_count = tokens.len();

    // 2. Parser Benchmark (Setup isolated: token clone happens outside timed & memory measurement)
    let parse_stat = run_bench_with_setup(
        "Parser::parse",
        Duration::from_millis(400),
        || tokens.clone(),
        |toks| {
            let mut parser = Parser::new(toks);
            parser.parse().unwrap()
        },
        |iters, dur| {
            let total_lines_processed = (source_lines * iters) as f64;
            let secs = dur.as_secs_f64();
            let lines_per_sec = total_lines_processed / secs;
            format!("{} lines/s", format_thousands(lines_per_sec as usize))
        },
    );

    // Pre-parse for Codegen & Analysis
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    // 3. ProgramInference Benchmark
    let infer_stat = run_bench(
        "ProgramInference::analyze",
        Duration::from_millis(400),
        || ProgramInference::analyze(&program),
        |iters, dur| {
            let secs = dur.as_secs_f64();
            let ops_per_sec = (iters as f64) / secs;
            format!("{} ops/s", format_thousands(ops_per_sec as usize))
        },
    );

    // 4. Codegen Benchmark (Host Architecture)
    let host_asm = generate(&program, host_arch, host_os);
    let host_out_len = host_asm.len();
    let host_out_lines = host_asm.lines().count();

    let host_label: &'static str = match host_arch {
        Architecture::X64 => "CodeGen::generate (Host: x64)",
        Architecture::ARM64 => "CodeGen::generate (Host: ARM64)",
        Architecture::X86 => "CodeGen::generate (Host: x86)",
    };

    let codegen_host_stat = run_bench(
        host_label,
        Duration::from_millis(400),
        || generate(&program, host_arch, host_os),
        |iters, dur| {
            let total_lines = (host_out_lines * iters) as f64;
            let secs = dur.as_secs_f64();
            let lines_per_sec = total_lines / secs;
            format!("{} asm lines/s", format_thousands(lines_per_sec as usize))
        },
    );

    // 5. Cross-Target Codegen Benchmark (ARM64)
    let arm_asm = generate(&program, Architecture::ARM64, OperatingSystem::Linux);
    let arm_out_lines = arm_asm.lines().count();

    let codegen_arm_stat = run_bench(
        "CodeGen::generate (Cross: ARM64)",
        Duration::from_millis(400),
        || generate(&program, Architecture::ARM64, OperatingSystem::Linux),
        |iters, dur| {
            let total_lines = (arm_out_lines * iters) as f64;
            let secs = dur.as_secs_f64();
            let lines_per_sec = total_lines / secs;
            format!("{} asm lines/s", format_thousands(lines_per_sec as usize))
        },
    );

    // 6. Full Frontend Pipeline Benchmark (Lex -> Parse -> Infer -> Codegen)
    let full_stat = run_bench(
        "Full Compiler Pipeline",
        Duration::from_millis(500),
        || {
            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize().unwrap();
            let mut parser = Parser::new(tokens);
            let prog = parser.parse().unwrap();
            let _ = ProgramInference::analyze(&prog);
            generate(&prog, host_arch, host_os)
        },
        |iters, dur| {
            let secs = dur.as_secs_f64();
            let ops_per_sec = (iters as f64) / secs;
            format!("{:.1} files/s", ops_per_sec)
        },
    );

    let baseline_mean_nanos = lex_stat.mean.as_nanos() as f64;
    let baseline_alloc = lex_stat.allocated_bytes.max(1) as f64;

    let results = [
        lex_stat,
        parse_stat,
        infer_stat,
        codegen_host_stat,
        codegen_arm_stat,
        full_stat,
    ];

    // BenchmarkDotNet Summary Table
    println!(
        "| {:<32} | {:>6} | {:>10} | {:>10} | {:>10} | {:>10} | {:>10} | {:>6} | {:>10} | {:>11} | {:>22} |",
        "Benchmark Stage", "Iters", "Mean", "Error", "StdDev", "Min", "Max", "Ratio", "Allocated", "Alloc Ratio", "Throughput"
    );
    println!(
        "|:{:-<32}-|-{:-<6}:|-{:-<10}:|-{:-<10}:|-{:-<10}:|-{:-<10}:|-{:-<10}:|-{:-<6}:|-{:-<10}:|-{:-<11}:|-{:-<22}:|",
        "", "", "", "", "", "", "", "", "", "", ""
    );

    for r in &results {
        let ratio = (r.mean.as_nanos() as f64) / baseline_mean_nanos;
        let alloc_ratio = (r.allocated_bytes as f64) / baseline_alloc;

        println!(
            "| {:<32} | {:>6} | {:>10} | {:>10} | {:>10} | {:>10} | {:>10} | {:>6.2} | {:>10} | {:>11.2} | {:>22} |",
            r.name,
            r.iterations,
            format_duration(r.mean),
            format_duration(r.error),
            format_duration(r.std_dev),
            format_duration(r.min),
            format_duration(r.max),
            ratio,
            format_bytes(r.allocated_bytes),
            alloc_ratio,
            r.throughput
        );
    }

    println!("\n// * Legends *");
    println!("  Mean        : Arithmetic mean of all measurements");
    println!("  Error       : Half of 99.9% confidence interval");
    println!("  StdDev      : Standard deviation of all measurements");
    println!("  Min / Max   : Minimum and maximum recorded execution time");
    println!("  Ratio       : Mean time ratio relative to baseline (Lexer::tokenize)");
    println!("  Allocated   : Allocated heap memory per single operation (1 KB = 1024 B)");
    println!("  Alloc Ratio : Allocated memory ratio relative to baseline");
    println!("  Throughput  : Processed workload units per second\n");

    println!(
        "Workload Metrics: {} tokens, {} lines generated ASM ({:.1} KB)",
        token_count,
        host_out_lines,
        host_out_len as f64 / 1024.0
    );
}
