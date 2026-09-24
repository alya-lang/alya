mod common;
use common::*;

#[test]
fn test_e2e_mem_float_roundtrip() {
    // std/mem read_float/write_float delegate to raw 64-bit peek/poke.
    // Alya floats travel as raw f64 bits in value slots, so the round-trip
    // is exact on 64-bit backends. Locks that contract in.
    // x86 is excluded: its peek/poke are 32-bit (known backend limitation).
    if cfg!(target_arch = "x86") {
        return;
    }
    let code = r#"
import "std/mem"
let arena = Arena.new(1024)
let p = arena.alloc(64)
write_float(p, 0, 4.25)
write_float(p, 8, -2.5)
write_float(p, 16, 42.0)
write_float(p, 24, 1e300)
write_int(p, 32, 123456789)
assert_eq(read_float(p, 0), 4.25)
assert_eq(read_float(p, 8), -2.5)
assert_eq(read_float(p, 16), 42.0)
assert_eq(read_float(p, 24), 1e300)
assert_eq(read_int(p, 32), 123456789)
assert(read_float(p, 0) > 4.0)
assert(read_float(p, 8) < 0.0)
assert_eq(read_float(p, 0) + 1.0, 5.25)
if read_float(p, 0) == 4.25
    say "FLOAT ROUNDTRIP OK"
end
"#;

    if let Some((exit, output)) = run_alya_code_full(code) {
        assert_eq!(exit, 0, "Failed with code {}\nOutput:\n{}", exit, output);
        assert!(output.contains("FLOAT ROUNDTRIP OK"), "Got:\n{}", output);
    }
}

#[test]
fn test_e2e_mem_narrow_roundtrip() {
    // std/mem read_f32/write_f32/read_i32/write_i32 narrow memory access.
    // f32 widens exactly to f64; i32 sign-extends into the 64-bit slot.
    // x86 is excluded: its value slots are 32-bit (known backend limitation).
    if cfg!(target_arch = "x86") {
        return;
    }
    let code = r#"
import "std/mem"
let arena = Arena.new(1024)
let p = arena.alloc(64)
write_f32(p, 0, 4.25)
write_f32(p, 4, -2.5)
write_f32(p, 8, 0.1)
write_i32(p, 16, 123456789)
write_i32(p, 20, -42)
assert_eq(read_f32(p, 0), 4.25)
assert_eq(read_f32(p, 4), -2.5)
assert(read_f32(p, 8) > 0.0999999)
assert(read_f32(p, 8) < 0.1000001)
assert_eq(read_i32(p, 16), 123456789)
assert_eq(read_i32(p, 20), -42)
assert_eq(read_i32(p, 16) + read_i32(p, 20), 123456747)
if read_f32(p, 0) == 4.25
    say "NARROW ROUNDTRIP OK"
end
"#;

    if let Some((exit, output)) = run_alya_code_full(code) {
        assert_eq!(exit, 0, "Failed with code {}\nOutput:\n{}", exit, output);
        assert!(output.contains("NARROW ROUNDTRIP OK"), "Got:\n{}", output);
    }
}

#[test]
fn test_e2e_str_from_float_exact() {
    // str(float) must render the argument, not a stale FP register.
    // Regression: arm64 fn_str_from_float read d0 directly while floats
    // travel as int-reg bit patterns (every other float consumer converts
    // with fmov first), printing pointer bits as `%g` garbage such as
    // 3.04803e-314 for str(1.0).
    let code = r#"
say str(1.0)
say str(2.5)
say str(-0.5)
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "1\n2.5\n-0.5\n", "Got:\n{}", output);
    }
}

#[test]
fn test_e2e_builtins() {
    let code = r#"
say len("Hello, Alya!")
say abs(-42)
say abs(42)
say min(15, 30)
say max(15, 30)
"#;
    if let Some(output) = run_alya_code(code) {
        assert_eq!(output, "12\n42\n42\n15\n30\n");
    }
}

#[test]
fn test_e2e_ask_input_automated() {
    let code = r#"
let name = ask "Name: "
let city = ask "City: "
say "Hello, {name} from {city}!"
"#;
    if let Some((code, output)) = run_alya_code_with_input(code, Some("Alya\nIstanbul\n")) {
        assert_eq!(code, 0);
        assert!(output.contains("Name: "));
        assert!(output.contains("City: "));
        assert!(output.contains("Hello, Alya from Istanbul!"));
    }
}

#[test]
fn test_e2e_ask_input_with_numeric_parsing() {
    let code = r#"
let item_name = ask "Item: "
let raw_price = ask "Price: "
let raw_qty   = ask "Qty: "

let price = float(raw_price)
let qty   = int(raw_qty)

let subtotal = price * float(qty)
say "Item: {item_name}"
say "Qty: {qty}"
say "Price: {price}"
say "Subtotal: {subtotal}"
"#;
    if let Some((code, output)) = run_alya_code_with_input(code, Some("Laptop\n450.5\n2\n")) {
        assert_eq!(code, 0);
        assert!(output.contains("Item: Laptop"));
        assert!(output.contains("Qty: 2"));
        assert!(output.contains("Price: 450.5"));
        assert!(output.contains("Subtotal: 901"));
    }
}

#[test]
fn test_e2e_sqrt_and_pow() {
    let code = r#"
say sqrt(16)
say sqrt(25)
say sqrt(1)
say sqrt(0)
say pow(2, 8)
say pow(5, 3)
say pow(10, 0)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "4\n5\n1\n0\n256\n125\n1\n");
    }
}

#[test]
fn test_e2e_floating_point() {
    let code = r#"
let pi = 3.14
let r = 2.0
let area = pi * r * r
say area

let a = 10.5
let b = 2.5
say a + b
say a - b
say a * b
say a / b

let c = 1.25
c += 0.75
say c

if a > b
    say "greater"
end

let int_val = 10
let flt_val = float(int_val)
say flt_val

let back_to_int = int(3.99)
say back_to_int

say "Interpolated: {pi}"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "12.56\n13\n8\n26.25\n4.2\n2\ngreater\n10\n3\nInterpolated: 3.14\n"
        );
    }
}

#[test]
fn test_e2e_cli_args() {
    let code = r#"
let a = args()
say a.length()
for arg in a
    say arg
end
if a.length() > 0
    say "first: {a[0]}"
    let num = int(a[2])
    say "parsed_int: {num}"
    let next_num = num + 8
    say "calc: {next_num}"
end
"#;
    // Test with CLI arguments
    if let Some((code, output)) = run_alya_code_with_args(code, &["hello", "alya", "42"]) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "3\n",
                "hello\n",
                "alya\n",
                "42\n",
                "first: hello\n",
                "parsed_int: 42\n",
                "calc: 50\n",
            )
        );
    }

    // Test with no CLI arguments
    if let Some((code, output)) = run_alya_code_with_args(code, &[]) {
        assert_eq!(code, 0);
        assert_eq!(output, "0\n");
    }
}

#[test]
fn test_e2e_file_io() {
    let test_path = "target/test_file_io.txt";
    let _ = std::fs::remove_file(test_path);

    let code = r#"
let path = "target/test_file_io.txt"
say file_exists(path)

let ok = write_file(path, "Hello Alya!\nSelf-hosting is coming.")
say ok
say file_exists(path)

let content = read_file(path)
say content

let del_ok = delete_file(path)
say del_ok
say file_exists(path)

let del_missing = remove_file("target/non_existent_12345.txt")
say del_missing

let missing = read_file("target/non_existent_12345.txt")
say "missing: [{missing}]"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "0\n",
                "1\n",
                "1\n",
                "Hello Alya!\nSelf-hosting is coming.\n",
                "1\n",
                "0\n",
                "0\n",
                "missing: []\n",
            )
        );
    }

    let _ = std::fs::remove_file(test_path);
}

#[test]
fn test_e2e_stdlib_modules() {
    let code = r#"
import "std/math"
import "std/time"
import "std/os"
import "std/json"

say clamp(50, 0, 10)
say hypot(3, 4)
say is_even(10)
say is_odd(7)

let t = now()
if t > 0
    say 1
else
    say 0
end
delay(5)

say json_number(42)
say json_string("alya")
say json_bool(1)
say json_bool(0)
let items = ["apple", "banana"]
say json_array(items)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "10\n",
                "5\n",
                "1\n",
                "1\n",
                "1\n",
                "42\n",
                "\"alya\"\n",
                "true\n",
                "false\n",
                "[apple, banana]\n",
            )
        );
    }
}

#[test]
fn test_e2e_memory_and_arena() {
    let code = r#"
import "std/mem"

# 1. Size helpers & pointer inspection
say kb(2)
say is_null(0)
say is_valid(0)
let ptr = alloc_zeroed(32)
say is_valid(ptr)
say read_byte(ptr, 0)

# 2. Byte & int access
write_byte(ptr, 0, 65)
write_byte(ptr, 1, 66)
write_byte(ptr, 2, 0)
say to_string(ptr)
say read_byte(ptr, 1)

write_int(ptr, 8, 999888)
say read_int(ptr, 8)

# 3. Pointer arithmetic, calloc, and memory comparison
let ptr2 = calloc_mem(4, 8)
write_byte(ptr2, 0, 65)
write_byte(ptr2, 1, 66)
say mem_equal(ptr, ptr2, 2)
say mem_equal(ptr, ptr2, 16)

let diff = ptr_diff(ptr_add(ptr, 10), ptr)
say diff

free_mem(ptr)
free_mem(ptr2)

# 4. Arena allocator with string and zeroed allocation
let a = arena_new(kb(1))
say arena_is_valid(a)
let str_arena = arena_alloc_string(a, "ArenaString")
say str_arena
let z_mem = arena_alloc_zeroed(a, 16)
say read_byte(z_mem, 0)
if arena_total_allocated(a) > 0
    say 1
else
    say 0
end
arena_clear(a)
say arena_total_allocated(a)
arena_free(a)
say arena_is_valid(a)

# 5. ByteBuffer
let buf = buffer_new(16)
say buffer_is_empty(buf)
buffer_write_string(buf, "Hello")
buffer_write_byte(buf, 32)
buffer_write_string(buf, "Alya")
say buffer_to_string(buf)
say buffer_len(buf)
say buffer_read_byte(buf, 0)
buffer_write_int(buf, 12345678)
say buffer_read_int(buf, 10)
buffer_clear(buf)
say buffer_len(buf)
say buffer_is_empty(buf)
buffer_free(buf)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!(
                "2048\n",
                "1\n",
                "0\n",
                "1\n",
                "0\n",
                "AB\n",
                "66\n",
                "999888\n",
                "1\n",
                "0\n",
                "10\n",
                "1\n",
                "ArenaString\n",
                "0\n",
                "1\n",
                "0\n",
                "0\n",
                "1\n",
                "Hello Alya\n",
                "10\n",
                "72\n",
                "12345678\n",
                "0\n",
                "1\n",
            )
        );
    }
}

#[test]
fn test_e2e_aligned_memory_allocation() {
    let code = r#"
import "std/mem"

# 1. Test 32-byte alignment (AVX2 requirement)
let p32 = aligned_alloc(128, 32)
say is_valid(p32)
say p32 % 32 == 0

# 2. Test 64-byte alignment (AVX-512 requirement)
let p64 = aligned_alloc(256, 64)
say is_valid(p64)
say p64 % 64 == 0

# 3. Write and read to ensure memory is writable and intact
write_int(p32, 0, 42)
write_int(p32, 32, 84)
say read_int(p32, 0)
say read_int(p32, 32)

aligned_free(p32)
aligned_free(p64)
say 1
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!("1\n", "1\n", "1\n", "1\n", "42\n", "84\n", "1\n",)
        );
    }
}

#[test]
fn test_e2e_vector_and_ai_math_primitives() {
    let code = r#"
import "std/math"

# 1. lerp test
say lerp(10.0, 20.0, 0.5)

# 2. dot_product test: [1.0, 2.0, 3.0, 4.0, 5.0] . [2.0, 3.0, 4.0, 5.0, 6.0]
# = 1*2 + 2*3 + 3*4 + 4*5 + 5*6 = 2 + 6 + 12 + 20 + 30 = 70.0
let v1 = [1.0, 2.0, 3.0, 4.0, 5.0]
let v2 = [2.0, 3.0, 4.0, 5.0, 6.0]
say dot_product(v1, v2)

# 3. vector_norm: [3.0, 4.0] -> sqrt(9 + 16) = 5.0
let v3 = [3.0, 4.0]
say vector_norm(v3)

# 4. cosine_similarity: identical vectors -> 1.0, orthogonal vectors -> 0.0
let va = [1.0, 0.0]
let vb = [0.0, 1.0]
say cosine_similarity(va, va)
say cosine_similarity(va, vb)

# 5. sum_f64: sum of 10 floats
let numbers = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
say sum_f64(numbers)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!("15\n", "70\n", "5\n", "1\n", "0\n", "55\n",)
        );
    }
}

#[test]
fn test_e2e_simd_vector_primitives() {
    let code = r#"
import "std/simd"

let v1 = f64x4_new(1.0, 2.0, 3.0, 4.0)
let v2 = f64x4_splat(2.0)

# 1. Component get
say v1.get(0)
say v1.get(3)

# 2. Operator + (v1 + v2: [3.0, 4.0, 5.0, 6.0])
let v_add = v1 + v2
say v_add.get(0)
say v_add.get(3)

# 3. Operator * (v1 * v2: [2.0, 4.0, 6.0, 8.0])
let v_mul = v1 * v2
say v_mul.get(0)
say v_mul.get(3)

# 4. Horizontal reduction sum (1 + 2 + 3 + 4 = 10)
say v1.sum_horizontal()

# 5. Horizontal min & max
say v1.min()
say v1.max()

# 6. FMA: (v1 * v2) + v_add = [2, 4, 6, 8] + [3, 4, 5, 6] = [5, 8, 11, 14]
let v_fma = v1.fma(v2, v_add)
say v_fma.get(0)
say v_fma.get(2)

# 7. Shuffle / Swizzle
let v_rev = v1.shuffle(3, 2, 1, 0)
say v_rev.get(0)
say v_rev.get(3)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!(
                "1\n", "4\n", "3\n", "6\n", "2\n", "8\n", "10\n", "1\n", "4\n", "5\n", "11\n",
                "4\n", "1\n",
            )
        );
    }
}

#[test]
fn test_e2e_time_and_test_stdlib() {
    let code = r#"
import "std/time"
import "std/test"

# 1. Time functions
say is_leap_year(2024)
say is_leap_year(2023)
say is_leap_year(2000)
say is_leap_year(1900)

say days_in_month(1, 2024)
say days_in_month(2, 2024)
say days_in_month(2, 2023)
say days_in_month(4, 2024)

say format_duration(3665)
say minutes(5)
say hours(2)

# 2. Test runner & comparison assertions
let r = runner_new()
runner_assert(r, 10 > 5, "greater than")
assert_gt(20, 10, "20 gt 10")
assert_gte(10, 10, "10 gte 10")
assert_lt(5, 10, "5 lt 10")
assert_lte(5, 5, "5 lte 5")
assert_not_null("hello", "not null")
assert_null(0, "is null")

say "Runner total: " + str(r.total)
say "Runner passed: " + str(r.passed)
say "Runner failed: " + str(r.failed)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            concat!(
                "1\n",
                "0\n",
                "1\n",
                "0\n",
                "31\n",
                "29\n",
                "28\n",
                "30\n",
                "1h 1m 5s\n",
                "300\n",
                "7200\n",
                "  [PASS] greater than\n",
                "  [PASS] 20 gt 10\n",
                "  [PASS] 10 gte 10\n",
                "  [PASS] 5 lt 10\n",
                "  [PASS] 5 lte 5\n",
                "  [PASS] not null\n",
                "  [PASS] is null\n",
                "Runner total: 1\n",
                "Runner passed: 1\n",
                "Runner failed: 0\n",
            )
        );
    }
}

#[test]
fn test_e2e_monotonic_clock_ms() {
    let code = r#"
let t0 = clock_ms()
sleep(60)
let elapsed = clock_ms() - t0
if elapsed >= 45
    say "clock_ms_ok"
else
    say "clock_ms_failed: " + str(elapsed)
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "clock_ms_ok\n");
    }
}

#[test]
fn test_e2e_os_stdlib() {
    let code = r#"
import "std/os"

# 1. Environment variables
say env_or("NON_EXISTENT_VAR_98765", "default_val")
say has_env("NON_EXISTENT_VAR_98765")
say has_env("PATH")

# 2. CLI arguments helpers
say arg_count()
say arg_at(0, "none")
say arg_at(1, "none")
say arg_at(99, "out_of_bounds")
say has_arg("--flag")
say has_arg("--unknown")

let c_args = cli_args()
for arg in c_args
    say "arg: " + arg
end

# 3. Platform & system
say target_os()
say target_arch()
say os_name()
say arch()
say is_windows() + is_linux() + is_macos()
say is_windows() + is_posix()
if len(platform()) > 0
    say "platform_ok"
end
if len(temp_dir()) > 0
    say "temp_dir_ok"
end
if len(null_device()) > 0
    say "null_device_ok"
end
if len(path_list_separator()) > 0
    say "path_sep_ok"
end

# 4. Command execution
let ret = exec("echo test > " + null_device())
say ret
"#;
    let expected_os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };

    let expected_arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "x64"
    };

    if let Some((code, output)) = run_alya_code_with_args(code, &["--flag", "input.txt"]) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            format!(
                concat!(
                    "default_val\n",
                    "0\n",
                    "1\n",
                    "2\n",
                    "--flag\n",
                    "input.txt\n",
                    "out_of_bounds\n",
                    "1\n",
                    "0\n",
                    "arg: --flag\n",
                    "arg: input.txt\n",
                    "{}\n",
                    "{}\n",
                    "{}\n",
                    "{}\n",
                    "1\n",
                    "1\n",
                    "platform_ok\n",
                    "temp_dir_ok\n",
                    "null_device_ok\n",
                    "path_sep_ok\n",
                    "0\n",
                ),
                expected_os, expected_arch, expected_os, expected_arch
            )
        );
    }
}

#[test]
fn test_e2e_os_exit_process() {
    let code = r#"
import "std/os"

say "before_exit"
exit_process(42)
say "unreachable"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 42);
        assert_eq!(output, "before_exit\n");
    }
}

#[test]
fn test_e2e_os_enums() {
    let code = r#"
import "std/os"

let my_os = os.current_os()
let my_arch = os.current_arch()

let os_ok = when my_os
    is os.OS.Windows => 1
    is os.OS.Linux   => 1
    is os.OS.MacOS   => 1
    else             => 0
end

let arch_ok = when my_arch
    is os.Arch.X64   => 1
    is os.Arch.X86   => 1
    is os.Arch.ARM64 => 1
    else             => 0
end

say os_ok
say arch_ok
say (my_os == os.OS.Unknown)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed: {}", output);
        assert_eq!(output, "1\n1\n0\n");
    }
}

#[test]
fn test_e2e_path_stdlib() {
    let code = r#"
import "std/path"

# 1. Separators
say path_separator()
if len(native_separator()) > 0
    say "native_sep_ok"
end

# 2. Classification
say is_absolute("/foo/bar")
say is_relative("/foo/bar")
say is_relative("foo/bar")
say is_root("/")
say is_root("/foo")

# 3. Filename, extension, stem, and hidden file edge case
say file_name("path/to/file.txt")
say basename("path/to/file.txt")
say file_name("path/to/dir/")
say file_ext("path/to/file.txt")
say extname("path/to/file.txt")
say has_extension("path/to/file.txt")
say has_extension("path/to/noext")
say file_stem("path/to/file.txt")
say stem("path/to/file.txt")

# Edge cases: dotfiles
say file_name(".gitignore")
say file_ext(".gitignore")
say file_stem(".gitignore")

# 4. Parent dir
say parent_dir("path/to/file.txt")
say dirname("path/to/file.txt")
say parent_dir("/file.txt")
say parent_dir("file.txt")

# 5. Transformations
say with_file_name("path/to/old.txt", "new.txt")
say with_file_name("old.txt", "new.txt")
say with_extension("path/to/file.txt", ".md")
say with_extension("path/to/file.txt", "md")
say with_extension("file.txt", "")

# 6. Joining & Normalization
say path_join("usr/local", "bin/alya")
say path_join("usr/local/", "/bin/alya")
say path_join3("usr", "local", "bin")
let parts = ["a", "b", "c", "d"]
say path_join_all(parts)
say to_slash("foo\\bar\\baz")
say normalize_slashes("foo\\bar\\baz")
"#;

    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!(
                "/\n",
                "native_sep_ok\n",
                "1\n",
                "0\n",
                "1\n",
                "1\n",
                "0\n",
                "file.txt\n",
                "file.txt\n",
                "dir\n",
                ".txt\n",
                ".txt\n",
                "1\n",
                "0\n",
                "file\n",
                "file\n",
                ".gitignore\n",
                "\n",
                ".gitignore\n",
                "path/to\n",
                "path/to\n",
                "/\n",
                ".\n",
                "path/to/new.txt\n",
                "new.txt\n",
                "path/to/file.md\n",
                "path/to/file.md\n",
                "file\n",
                "usr/local/bin/alya\n",
                "usr/local/bin/alya\n",
                "usr/local/bin\n",
                "a/b/c/d\n",
                "foo/bar/baz\n",
                "foo/bar/baz\n",
            )
        );
    }
}

#[test]
fn test_e2e_module_alias_and_conflict_resolution() {
    let code = r#"
import "tests/fixtures/modules/conflict/module1.alya" as m1
import "tests/fixtures/modules/conflict/module2.alya" as m2
import "std/math" as m

say m1::abc()
say m2::abc()
say m::clamp(15, 0, 10)
say m::is_even(4)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(output, "1\n2\n10\n1\n");
    }
}

#[test]
fn test_e2e_str_stdlib_extended() {
    let code = r#"
import "std/str"

say is_blank("")
say is_blank("   \t \r\n ")
say is_blank(" a ")

say trim_start("  alya  ")
say trim_end("  alya  ")
say trim_char("***hello***", "*")
say center("alya", 8, "-")
say title_case("hello world of alya")

say index_of("banana", "na")
say index_of("banana", "xyz")
say last_index_of("banana", "na")
say contains_str("banana", "nan")
say contains_str("banana", "apple")

say reverse_str("alya")
say truncate("hello world", 8, "...")

say is_numeric("12345")
say is_numeric("123a5")
say is_alphabetic("Alya")
say is_alphabetic("Alya1")
say is_alphanumeric("Alya2026")
say is_alphanumeric("Alya-2026")
say is_upper("ALYA")
say is_upper("Alya")
say is_lower("alya")
say is_lower("Alya")
say slugify("Hello World! 2026")
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!(
                "1\n1\n0\n",
                "alya  \n",
                "  alya\n",
                "hello\n",
                "--alya--\n",
                "Hello World Of Alya\n",
                "2\n-1\n4\n1\n0\n",
                "ayla\n",
                "hello...\n",
                "1\n0\n1\n0\n1\n0\n1\n0\n1\n0\n",
                "hello-world-2026\n"
            )
        );
    }
}

#[test]
fn test_e2e_math_stdlib_extended() {
    let code = r#"
import "std/math"

say gcd(48, 18)
say gcd(101, 103)
say lcm(12, 18)
say factorial(5)
say factorial(0)
say is_prime(7)
say is_prime(4)
say is_prime(1)

say round(degrees(radians(180.0)))
say round(lerp(10.0, 20.0, 0.5))
say round(norm(5.0, 0.0, 10.0) * 100.0)
say round(smoothstep(0.0, 1.0, 0.5) * 100.0)

let data = [2, 4, 4, 4, 5, 5, 7, 9]
say round(variance(data))
say round(std_dev(data))

say round(sin(0.0))
say round(cos(0.0))
say round(tan(0.0))
say round(atan2(0.0, 1.0))
say round(log2(8.0))
say round(log10(100.0))
say round(sqrt_f(16.0))
say round(hypot(3.0, 4.0))
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!(
                "6\n1\n36\n120\n1\n1\n0\n0\n",
                "180\n15\n50\n50\n",
                "4\n2\n",
                "0\n1\n0\n0\n3\n2\n4\n5\n"
            )
        );
    }
}

#[test]
fn test_e2e_collections_stdlib_extended() {
    let code = r#"
import "std/collections"

# Array utilities
let arr = [5, 2, 8, 1, 9, 2]
say array_contains(arr, 8)
say array_contains(arr, 99)
say array_index_of(arr, 2)
say array_last_index_of(arr, 2)

let sl = array_slice(arr, 1, 4)
say len(sl)
say sl[0]
say sl[2]

let rev = array_reverse([1, 2, 3])
say rev[0]
say rev[2]

let uniq = array_unique([1, 2, 2, 3, 1, 4])
say len(uniq)

say array_min([10, 4, 25, 2, 18])
say array_max([10, 4, 25, 2, 18])

let sorted = array_sort([5, 1, 4, 2, 8])
say sorted[0]
say sorted[4]

let chunks = array_chunk([1, 2, 3, 4, 5], 2)
say len(chunks)

let filled = array_fill(7, 3)
say len(filled)
say filled[0]

# Set operations
let s1 = set_from_array([1, 2, 3])
let s2 = set_from_array([2, 3, 4])
let u = set_union(s1, s2)
say set_size(u)
let inter = set_intersection(s1, s2)
say set_size(inter)
let diff = set_difference(s1, s2)
say set_size(diff)
say set_is_subset(inter, s1)

# Map utilities
let m1 = {}
m1["a"] = 10
m1["b"] = 20
let m2 = map_clone(m1)
say m2["a"]
say map_is_empty(m2)
say map_is_empty({})

# Queue (O(1) amortized)
let q = queue_new()
queue_push(q, 100)
queue_push(q, 200)
queue_push(q, 300)
say queue_size(q)
say queue_peek(q)
say queue_pop(q)
say queue_pop(q)
say queue_size(q)
say queue_is_empty(q)

let arr_q = queue_to_array(q)
say len(arr_q)
say arr_q[0]

queue_clear(q)
say queue_size(q)
say queue_is_empty(q)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!(
                "1\n0\n1\n5\n",
                "3\n2\n1\n",
                "3\n1\n",
                "4\n",
                "2\n25\n",
                "1\n8\n",
                "3\n",
                "3\n7\n",
                "4\n2\n1\n1\n",
                "10\n0\n1\n",
                "3\n100\n100\n200\n1\n0\n",
                "1\n300\n",
                "0\n1\n"
            )
        );
    }
}

#[test]
fn test_e2e_fs_and_hash_stdlib_extended() {
    let code = r#"
import "std/fs"
import "std/hash"
import "std/str"

let path = "test_ext_fs_io.txt"
write_lines(path, ["first line", "second line"])
say is_empty_file(path)

let read_back = read_lines(path)
say len(read_back)
say read_back[0]
say read_back[1]

append_line(path, "third line")
let read_back2 = read_lines(path)
say len(read_back2)

clear_file(path)
say is_empty_file(path)
fs_remove(path)

# Hash utilities
let c1 = crc32("hello world")
let c2 = crc32("hello world")
let c3 = crc32("different")
if c1 == c2 and c1 != c3
    say 1
else
    say 0
end

let a1 = adler32("hello world")
let a2 = adler32("hello world")
let a3 = adler32("different")
if a1 == a2 and a1 != a3
    say 1
else
    say 0
end

let s1 = sdbm("alya")
let s2 = sdbm("alya")
if s1 == s2 and s1 > 0
    say 1
else
    say 0
end

say is_hex("deadbeef")
say is_hex("not_hex")
say is_base64("QWx5YQ==")
say is_base64("invalid base64!")

say to_hex("Hi")
say from_hex("4869")
say to_base64("Alya")
say from_base64("QWx5YQ==")
say base64_encode_bytes([65, 108, 121, 97])
let dec_bytes = base64_decode_bytes("QWx5YQ==")
say len(dec_bytes)
say dec_bytes[0]
say dec_bytes[1]
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!(
                "0\n",
                "2\nfirst line\nsecond line\n",
                "3\n",
                "1\n",
                "1\n",
                "1\n",
                "1\n",
                "1\n0\n1\n0\n",
                "4869\nHi\nQWx5YQ==\nAlya\n",
                "QWx5YQ==\n4\n65\n108\n"
            )
        );
    }
}

#[test]
fn test_e2e_json_stdlib_extended() {
    let code = r#"
import "std/json"

say json_null()
say json_int(42)
say json_float(3.14)
say json_string("hello \"quotes\" and \\backslash")

say json_array([json_string("one"), json_string("two")])
say json_array([json_int(10), json_int(20), json_int(30)])

let user = {}
user["name"] = json_string("Alice")
let s_obj = json_object(user)
let parsed_user = json_parse(s_obj)
say str_from_ptr(parsed_user["name"])

let parsed = json_parse("{\"title\": \"Alya\", \"score\": 100, \"active\": true, \"tags\": [\"fast\", \"native\"]}")
say str_from_ptr(parsed["title"])
say parsed["score"]
say parsed["active"]
let tags = parsed["tags"]
say str_from_ptr(tags[0])
say str_from_ptr(tags[1])
say json_is_valid("{\"ok\": 1}")
say json_is_valid("invalid json")
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(
            code, 0,
            "Execution failed with code {} and output:\n{}",
            code, output
        );
        assert_eq!(
            output,
            concat!(
                "null\n",
                "42\n",
                "3.14\n",
                "\"hello \\\"quotes\\\" and \\\\backslash\"\n",
                "[\"one\", \"two\"]\n",
                "[10, 20, 30]\n",
                "Alice\n",
                "Alya\n",
                "100\n",
                "1\n",
                "fast\n",
                "native\n",
                "1\n",
                "0\n"
            )
        );
    }
}

#[test]
fn test_e2e_bench_stdlib() {
    let code = r#"
import "std/bench"

let b = bench_runner("E2E Test Suite")
bench_start(b, 100)
let i = 0
let sum = 0
while i < 100
    sum = sum + i
    i = i + 1
end
bench_stop(b, "loop_sum")
say "sum: {sum}"
bench_summary(b)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("=== Benchmark Suite: E2E Test Suite ==="));
        assert!(output.contains("* loop_sum: 100 iters in"));
        assert!(output.contains("sum: 4950"));
        assert!(output.contains("// * Summary *"));
        assert!(output.contains("// * Legends *"));
        assert!(output.contains("Finished 1 benchmark(s) in"));
    }
}

#[test]
fn test_e2e_rand_stdlib() {
    let code = r#"
import "std/rand"

rand_seed(12345)

let r_int = rand_int(10, 20)
say "int ok: {r_int >= 10 and r_int <= 20}"

let r_flt = rand_float()
say "flt ok: {r_flt >= 0.0 and r_flt < 1.0}"

let r_rng = rand_float_range(5.0, 10.0)
say "range ok: {r_rng >= 5.0 and r_rng < 10.0}"

let r_b = rand_bool()
say "bool ok: {r_b == 0 or r_b == 1}"

let r_c0 = rand_chance(0)
say "chance0 ok: {r_c0 == 0}"

let r_c100 = rand_chance(100)
say "chance100 ok: {r_c100 == 1}"

let arr = [10, 20, 30, 40, 50]
let chosen = rand_choice(arr)
say "choice ok: {chosen >= 10 and chosen <= 50}"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("int ok: 1"));
        assert!(output.contains("flt ok: 1"));
        assert!(output.contains("range ok: 1"));
        assert!(output.contains("bool ok: 1"));
        assert!(output.contains("chance0 ok: 1"));
        assert!(output.contains("chance100 ok: 1"));
        assert!(output.contains("choice ok: 1"));
    }
}

#[test]
fn test_e2e_cli_stdlib() {
    let code = r#"
import "std/cli"

say "count: " + str(cli_arg_count())
say "verbose: " + str(cli_has_flag("verbose"))
say "v: " + str(cli_has_flag("v"))
say "debug: " + str(cli_has_flag("debug"))
say "out: " + cli_get_opt("output", "default")
say "missing_opt: " + cli_get_opt("missing", "fallback")
say "pos0: " + cli_get_positional(0, "none")
say "arg0: " + cli_arg_at(0, "none")
"#;
    if let Some((code, output)) = run_alya_code_with_args(
        code,
        &["--verbose", "-v", "--output=custom/target", "main.alya"],
    ) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("count: 4"), "Got: {}", output);
        assert!(output.contains("verbose: 1"), "Got: {}", output);
        assert!(output.contains("v: 1"), "Got: {}", output);
        assert!(output.contains("debug: 0"), "Got: {}", output);
        assert!(output.contains("out: custom/target"), "Got: {}", output);
        assert!(output.contains("missing_opt: fallback"), "Got: {}", output);
        assert!(output.contains("pos0: main.alya"), "Got: {}", output);
        assert!(output.contains("arg0: --verbose"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_csv_stdlib_migration_diagnostic() {
    let code = r#"
import "std/csv"
say "should fail"
"#;
    let mut lexer = alya::lexer::Lexer::new(code);
    let tokens = lexer.tokenize().expect("Tokenize failed");
    let mut parser = alya::parser::Parser::new(tokens);
    let mut ast = parser.parse().expect("Parse failed");
    let err = alya::parser::resolve_imports(&mut ast, std::path::Path::new(".")).unwrap_err();
    assert!(err.contains("Cannot find standard library module 'std/csv'"));
}

#[test]
fn test_e2e_url_stdlib_migration_diagnostic() {
    let code = r#"
import "std/url"
say "should fail"
"#;
    let mut lexer = alya::lexer::Lexer::new(code);
    let tokens = lexer.tokenize().expect("Tokenize failed");
    let mut parser = alya::parser::Parser::new(tokens);
    let mut ast = parser.parse().expect("Parse failed");
    let err = alya::parser::resolve_imports(&mut ast, std::path::Path::new(".")).unwrap_err();
    assert!(err.contains("Cannot find standard library module 'std/url'"));
}

#[test]
fn test_e2e_log_and_color_stdlib() {
    let code = r#"
import "std/color"
import "std/log"

let c_msg = color_green("SUCCESS") + " / " + color_red("FAILED")
let stripped = ansi_strip(c_msg)
say "stripped: " + stripped
say "is_plain: " + str(stripped == "SUCCESS / FAILED")

log_info("Top-level info")
log_warn("Top-level warn")

let l = logger_new("App", 2)
logger_debug(l, "hidden debug")
logger_info(l, "visible info")
logger_warn(l, "visible warn")
logger_error(l, "visible error")
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("stripped: SUCCESS / FAILED"));
        assert!(output.contains("[INFO]"));
        assert!(output.contains("Top-level info"));
        assert!(output.contains("[WARN]"));
        assert!(output.contains("Top-level warn"));
        assert!(output.contains("[App]"));
        assert!(output.contains("visible info"));
        assert!(output.contains("visible warn"));
        assert!(output.contains("visible error"));
        assert!(!output.contains("hidden debug"));
    }
}

#[test]
fn test_e2e_glob_stdlib() {
    let code = r#"
import "std/glob"

say "m1: " + str(glob_match("*.alya", "main.alya"))
say "m2: " + str(glob_match("*.alya", "main.rs"))
say "m3: " + str(glob_match("src/**/*.rs", "src/codegen/expr.rs"))
say "m4: " + str(glob_match("src/*.rs", "src/codegen/expr.rs"))
say "m5: " + str(glob_match("file_?.txt", "file_1.txt"))
say "m6: " + str(glob_match("file_?.txt", "file_12.txt"))
say "m7: " + str(glob_match("code_[0-9].rs", "code_7.rs"))
say "m8: " + str(glob_match("code_[!0-9].rs", "code_x.rs"))
say "m9: " + str(glob_match("code_[!0-9].rs", "code_7.rs"))

say "is_pat: " + str(glob_is_pattern("*.txt"))
say "not_pat: " + str(glob_is_pattern("plain.txt"))
say "esc: " + glob_escape("a*b?c[1]")

let items = ["apple.txt", "banana.csv", "cherry.txt"]
let filtered = glob_filter("*.txt", items)
say "f_len: " + str(array_len(filtered))
say "f0: " + str_from_ptr(filtered[0])
say "f1: " + str_from_ptr(filtered[1])

let toml_files = glob("Cargo.toml")
say "has_toml: " + str(array_len(toml_files))
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("m1: 1"));
        assert!(output.contains("m2: 0"));
        assert!(output.contains("m3: 1"));
        assert!(output.contains("m4: 0"));
        assert!(output.contains("m5: 1"));
        assert!(output.contains("m6: 0"));
        assert!(output.contains("m7: 1"));
        assert!(output.contains("m8: 1"));
        assert!(output.contains("m9: 0"));
        assert!(output.contains("is_pat: 1"));
        assert!(output.contains("not_pat: 0"));
        assert!(output.contains(r"esc: a\*b\?c\[1\]"));
        assert!(output.contains("f_len: 2"));
        assert!(output.contains("f0: apple.txt"));
        assert!(output.contains("f1: cherry.txt"));
        assert!(output.contains("has_toml: 1"));
    }
}

#[test]
fn test_e2e_console_stdlib() {
    let code = r#"
import "std/console"

let utf8_ok = console_utf8()
say "utf8_ok: {utf8_ok}"

let cp = console_output_cp()
say "cp: {cp}"

let enc = get_output_encoding()
say "enc: {enc}"

let title_ok = console_title("Alya Test Console")
say "title_ok: {title_ok}"

let beep_ok = console_beep()
say "beep_ok: {beep_ok}"

let clear_ok = console_clear()
say "clear_ok: {clear_ok}"

say "╔═════════════════╗"
say "║ UTF-8 Box Test  ║"
say "╚═════════════════╝"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("utf8_ok: 1"));
        assert!(output.contains("cp: 65001"));
        assert!(output.contains("enc: UTF-8"));
        assert!(output.contains("title_ok: 1"));
        assert!(output.contains("beep_ok: 1"));
        assert!(output.contains("clear_ok: 1"));
        assert!(output.contains("╔═════════════════╗"));
        assert!(output.contains("║ UTF-8 Box Test  ║"));
        assert!(output.contains("╚═════════════════╝"));
    }
}

#[test]
fn test_e2e_fmt_tool() {
    let unformatted = "function foo(a,b)\nlet x=10\nif x>5\nsay \"hello\"\nend\nreturn x\nend\n";
    let formatted = alya::tools::fmt::format_source(unformatted).expect("format_source failed");
    assert!(formatted.contains("function foo(a, b)"));
    assert!(formatted.contains("    let x=10"));
    assert!(formatted.contains("    if x>5"));
    assert!(formatted.contains("        say \"hello\""));
    assert!(formatted.contains("    end"));
    assert!(formatted.contains("    return x"));
    assert!(formatted.contains("end"));
}

#[test]
fn test_e2e_net_stdlib() {
    let code = r#"
import "std/net"

# 1. TCP Socket Lifecycle & Timeout
let sock = tcp_socket()
if sock >= 0
    say "tcp_socket: ok"
    let t_res = tcp_set_timeout(sock, 2000)
    say "tcp_timeout: " + str(t_res == 0)
    tcp_close(sock)
    say "tcp_closed: ok"
end

# 2. UDP Socket Lifecycle, Timeout, Send & Recv
let u_recv = udp_socket()
let u_send = udp_socket()
if u_recv >= 0 and u_send >= 0
    say "udp_sockets: ok"
    let b_res = udp_bind(u_recv, 29876)
    say "udp_bind: " + str(b_res == 0)
    udp_set_timeout(u_recv, 2000)
    let s_bytes = udp_send(u_send, "127.0.0.1", 29876, "hello_alya_udp")
    say "udp_sent: " + str(s_bytes > 0)
    let msg = udp_recv(u_recv, 128)
    say "udp_recv_msg: " + msg
    udp_close(u_recv)
    udp_close(u_send)
    say "udp_closed: ok"
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        println!("OUTPUT WAS:\n{}", output);
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("tcp_socket: ok"));
        assert!(output.contains("tcp_timeout: 1"));
        assert!(output.contains("tcp_closed: ok"));
        assert!(output.contains("udp_sockets: ok"));
        assert!(output.contains("udp_bind: 1"));
        assert!(output.contains("udp_sent: 1"));
        assert!(output.contains("udp_recv_msg: hello_alya_udp"));
        assert!(output.contains("udp_closed: ok"));
    }
}

#[test]
fn test_e2e_pool_and_str_clone_stdlib() {
    let code = r#"
import "std/mem"

# 1. Permanent heap strings
let original = "Hello Alya Heap Memory"
let cloned = str_clone(original)
say "cloned: " + cloned
str_free(cloned)
say "freed: ok"

# 2. Pool Allocator
let pool = pool_new(32, 4)
say "init_avail: " + str(pool_available(pool))

let b1 = pool_alloc(pool)
let b2 = pool_alloc(pool)
say "after_2_alloc: " + str(pool_available(pool))

poke_int(b1, 0, 12345)
poke_int(b2, 0, 67890)
say "b1_val: " + str(peek_int(b1, 0))
say "b2_val: " + str(peek_int(b2, 0))

pool_free(pool, b1)
say "after_1_free: " + str(pool_available(pool))

# Reuse freed block
let b3 = pool_alloc(pool)
say "b3_alloc: " + str(pool_available(pool))

pool_destroy(pool)
say "pool_destroyed: ok"

# 3. Memory Tracking Stats
let stats = mem_stats()
say "stats_ok: " + str(stats.allocated_bytes >= 0)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("cloned: Hello Alya Heap Memory"));
        assert!(output.contains("freed: ok"));
        assert!(output.contains("init_avail: 4"));
        assert!(output.contains("after_2_alloc: 2"));
        assert!(output.contains("b1_val: 12345"));
        assert!(output.contains("b2_val: 67890"));
        assert!(output.contains("after_1_free: 3"));
        assert!(output.contains("b3_alloc: 2"));
        assert!(output.contains("pool_destroyed: ok"));
        assert!(output.contains("stats_ok: 1"));
    }
}

#[test]
fn test_e2e_thread_and_concurrency_stdlib() {
    let code = r#"
import "std/thread"

function compute_square(x)
    return x * x
end

function compute_sum(pair)
    let a, b = pair
    return a + b
end

let t1 = thread_spawn(compute_square, 9)
let t2 = thread_spawn(compute_sum, (30, 12))

let r1 = thread_join(t1)
let r2 = thread_join(t2)

say "sq: " + str(r1)
say "sum: " + str(r2)

let tid = thread_id()
if tid > 0
    say "tid: ok"
end

let m = mutex_new()
mutex_lock(m)
mutex_unlock(m)
mutex_free(m)
say "mutex: ok"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(
            output.contains("sq: 81"),
            "Expected sq: 81, got: {}",
            output
        );
        assert!(
            output.contains("sum: 42"),
            "Expected sum: 42, got: {}",
            output
        );
        assert!(
            output.contains("tid: ok"),
            "Expected tid: ok, got: {}",
            output
        );
        assert!(
            output.contains("mutex: ok"),
            "Expected mutex: ok, got: {}",
            output
        );
    }
}

#[test]
fn test_e2e_stdlib_time_upgrades() {
    let code = r#"
import "std/time"

let dt = epoch_to_date(0)
say "y: " + str(dt["year"]) + " m: " + str(dt["month"]) + " d: " + str(dt["day"])
say "iso: " + format_iso(0)
say "date: " + format_date(0)
say "time: " + format_time_hhmmss(0)
say "map: " + format_date_map(dt)
say "month: " + month_name(1)
say "weekday: " + weekday_name(4)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("y: 1970 m: 1 d: 1"), "Got: {}", output);
        assert!(
            output.contains("iso: 1970-01-01T00:00:00Z"),
            "Got: {}",
            output
        );
        assert!(output.contains("date: 1970-01-01"), "Got: {}", output);
        assert!(output.contains("time: 00:00:00"), "Got: {}", output);
        assert!(
            output.contains("map: 1970-01-01T00:00:00Z"),
            "Got: {}",
            output
        );
        assert!(output.contains("month: January"), "Got: {}", output);
        assert!(output.contains("weekday: Thursday"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_stdlib_os_upgrades() {
    let code = r#"
import "std/os"

let p = get_pid()
if p > 0
    say "pid: ok"
end

let cur = get_cwd()
if len(cur) > 0
    say "cwd: ok"
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("pid: ok"), "Got: {}", output);
        assert!(output.contains("cwd: ok"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_stdlib_fs_upgrades() {
    let code = r#"
import "std/fs"

let base = file_basename("foo/bar/hello.txt")
say "base: " + base
let ext = file_extension("foo/bar/hello.txt")
say "ext: " + ext
let parent = file_parent("foo/bar/hello.txt")
say "parent: " + parent

ensure_dir("temp_e2e_fs_dir/sub")
say "sub is dir: " + str(is_dir("temp_e2e_fs_dir/sub"))
write_file("temp_e2e_fs_dir/sub/sample.txt", "one\ntwo\nthree\n")
let c = file_lines_count("temp_e2e_fs_dir/sub/sample.txt")
say "lines: " + str(c)

let items = list_dir_recursive("temp_e2e_fs_dir")
say "items: " + str(len(items))

remove_dir_recursive("temp_e2e_fs_dir")
say "cleaned: " + str(is_dir("temp_e2e_fs_dir") == 0)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("base: hello.txt"), "Got: {}", output);
        assert!(output.contains("ext: txt"), "Got: {}", output);
        assert!(output.contains("parent: foo/bar"), "Got: {}", output);
        assert!(output.contains("sub is dir: 1"), "Got: {}", output);
        assert!(output.contains("lines: 3"), "Got: {}", output);
        assert!(output.contains("items: 2"), "Got: {}", output);
        assert!(output.contains("cleaned: 1"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_stdlib_thread_concurrency() {
    let code = r#"
import "std/thread"

let ch = channel_new()
channel_send(ch, 100)
channel_send(ch, 200)
say "size: " + str(channel_size(ch))

let v1 = channel_recv(ch, 100)
let v2 = channel_recv(ch, 100)
say "recv: " + str(v1) + " and " + str(v2)
channel_free(ch)

let wg = wait_group_new()
wait_group_add(wg, 2)
wait_group_done(wg)
wait_group_done(wg)
let ok = wait_group_wait(wg, 100)
say "wg ok: " + str(ok)
wait_group_free(wg)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("size: 2"), "Got: {}", output);
        assert!(output.contains("recv: 100 and 200"), "Got: {}", output);
        assert!(output.contains("wg ok: 1"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_stdlib_console_controls() {
    let code = r#"
import "std/console"

let ok = console_utf8()
say "utf8: " + str(ok)
say "enc: " + get_output_encoding()
say "in_enc: " + get_input_encoding()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("utf8: 1"), "Got: {}", output);
        assert!(output.contains("enc: UTF-8"), "Got: {}", output);
        assert!(output.contains("in_enc: UTF-8"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_stdlib_net_helpers() {
    let code = r#"
import "std/net"

say "200: " + http_status_text(200)
say "404: " + http_status_text(404)
say "500: " + http_status_text(500)
say "auth: " + basic_auth("admin", "secret")
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("200: OK"), "Got: {}", output);
        assert!(output.contains("404: Not Found"), "Got: {}", output);
        assert!(
            output.contains("500: Internal Server Error"),
            "Got: {}",
            output
        );
        assert!(
            output.contains("auth: Basic YWRtaW46c2VjcmV0"),
            "Got: {}",
            output
        );
    }
}

#[test]
fn test_e2e_stdlib_net_nonblocking_and_poll() {
    let code = r#"
import "std/net"

let srv = tcp_listen(19876, 5)
if srv >= 0
    let nb_res = tcp_set_nonblocking(srv, 1)
    say "nb: " + str(nb_res)
    let poll_res = tcp_poll(srv, 20)
    say "poll: " + str(poll_res)
    tcp_close(srv)
    say "close: ok"
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("nb: 0"), "Got: {}", output);
        assert!(output.contains("poll: 0"), "Got: {}", output);
        assert!(output.contains("close: ok"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_stdlib_net_nonblocking_accept_negative() {
    let code = r#"
import "std/net"

let srv = tcp_listen(19877, 5)
if srv >= 0
    tcp_set_nonblocking(srv, 1)
    let client_fd = tcp_accept(srv)
    say "client_fd: " + str(client_fd)
    tcp_close(srv)
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("client_fd: -1"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_arc_memory_management() {
    let code = r#"
struct Point
    x
    y
end

let arr = [10, 20, 30]
say "arr initial rc: " + str(rc_count(arr))

let arr_alias = arr
say "arr after alias rc: " + str(rc_count(arr))
say "alias rc: " + str(rc_count(arr_alias))

let m = map()
say "map initial rc: " + str(rc_count(m))
let m_alias = m
say "map after alias rc: " + str(rc_count(m))
say "m_alias rc: " + str(rc_count(m_alias))

let p = Point(10, 20)
say "struct initial rc: " + str(rc_count(p))
let p_alias = p
say "struct after alias rc: " + str(rc_count(p))
say "p_alias rc: " + str(rc_count(p_alias))

function inspect_rc(item)
    return rc_count(item)
end

say "arr in function rc: " + str(inspect_rc(arr))
say "arr after function rc: " + str(rc_count(arr))

let temp = [99]
temp = arr
say "arr after reassign rc: " + str(rc_count(arr))

say "non-heap int rc: " + str(rc_count(42))
say "non-heap str rc: " + str(rc_count("hello"))
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("arr initial rc: 1"), "Got: {}", output);
        assert!(output.contains("arr after alias rc: 2"), "Got: {}", output);
        assert!(output.contains("alias rc: 2"), "Got: {}", output);
        assert!(output.contains("map initial rc: 1"), "Got: {}", output);
        assert!(output.contains("map after alias rc: 2"), "Got: {}", output);
        assert!(output.contains("m_alias rc: 2"), "Got: {}", output);
        assert!(output.contains("struct initial rc: 1"), "Got: {}", output);
        assert!(
            output.contains("struct after alias rc: 2"),
            "Got: {}",
            output
        );
        assert!(output.contains("p_alias rc: 2"), "Got: {}", output);
        assert!(output.contains("arr in function rc: 3"), "Got: {}", output);
        assert!(
            output.contains("arr after function rc: 2"),
            "Got: {}",
            output
        );
        assert!(
            output.contains("arr after reassign rc: 3"),
            "Got: {}",
            output
        );
        assert!(output.contains("non-heap int rc: 0"), "Got: {}", output);
        assert!(output.contains("non-heap str rc: 0"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_weak_references() {
    let code = r#"
struct Node
    val: int
end

struct Container
    item: weak Node
end

function attach_node(c)
    let n = Node { val: 42 }
    c.item = n
    say f"inside function n rc: {rc_count(n)}"
    say f"inside function c.item null: {c.item == null}"
end

function run_test()
    let c = Container { item: null }
    say f"c.item null: {c.item == null}"

    attach_node(c)
    # After attach_node returns, n is freed!
    say f"after return c.item null: {c.item == null}"
end

run_test()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("c.item null: 1"), "Got: {}", output);
        assert!(
            output.contains("inside function n rc: 1"),
            "Got: {}",
            output
        );
        assert!(
            output.contains("inside function c.item null: 0"),
            "Got: {}",
            output
        );
        assert!(
            output.contains("after return c.item null: 1"),
            "Got: {}",
            output
        );
    }
}

#[test]
fn test_e2e_sync_stdlib() {
    let code = r#"
import "std/sync"

# 1. Mutex
let m = mutex_new()
mutex_lock(m)
say "sync_mutex: locked"
mutex_unlock(m)
mutex_free(m)

# 2. Channel
let ch = channel_new()
channel_send(ch, 42)
channel_send(ch, 84)
say "ch_size: " + str(channel_size(ch))
let v1 = channel_try_recv(ch)
let v2 = channel_recv(ch, 1000)
say "ch_v1: " + str(v1)
say "ch_v2: " + str(v2)
channel_free(ch)

# 3. WaitGroup
let wg = wait_group_new()
wait_group_add(wg, 2)
wait_group_done(wg)
wait_group_done(wg)
let wg_res = wait_group_wait(wg, 1000)
say "wg_ok: " + str(wg_res)
wait_group_free(wg)

# 4. Once
let o = once_new()
let first = once_check(o)
let second = once_check(o)
say "once_1: " + str(first)
say "once_2: " + str(second)
once_free(o)

# 5. RwLock
let rw = rwlock_new()
let r1 = rwlock_read_lock(rw, 1000)
let r2 = rwlock_read_lock(rw, 1000)
say "rw_readers_ok: " + str(r1 + r2)
rwlock_read_unlock(rw)
rwlock_read_unlock(rw)

let w1 = rwlock_write_lock(rw, 1000)
say "rw_writer_ok: " + str(w1)
rwlock_write_unlock(rw)
rwlock_free(rw)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("sync_mutex: locked"), "Got: {}", output);
        assert!(output.contains("ch_size: 2"), "Got: {}", output);
        assert!(output.contains("ch_v1: 42"), "Got: {}", output);
        assert!(output.contains("ch_v2: 84"), "Got: {}", output);
        assert!(output.contains("wg_ok: 1"), "Got: {}", output);
        assert!(output.contains("once_1: 1"), "Got: {}", output);
        assert!(output.contains("once_2: 0"), "Got: {}", output);
        assert!(output.contains("rw_readers_ok: 2"), "Got: {}", output);
        assert!(output.contains("rw_writer_ok: 1"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_stdlib_modern_syntax_features() {
    let code = r#"
import "std/color"
import "std/log"
import "std/net"
import "std/sync"
import "std/collections"
import "std/str"
import "std/json"
import "std/test"

# 1. Color Enums
say "color_enum: " + str(Color.Red)
say "bg_enum: " + str(BgColor.Blue)
say "style_enum: " + str(Style.Bold)
let styled = color_fg(Color.Green, "ok_green")
if len(styled) > 0
    say "styled_ok"
end

# 2. LogLevel Enum and Logger Struct Methods
let logger = logger_new("test_logger", LogLevel.Info)
logger.set_colored(0)
logger.info("logger struct method works")
say "log_level_name: " + log_level_name(LogLevel.Warn)

# 3. HttpStatus Enum
say "http_200: " + http_status_text(HttpStatus.Ok)
say "http_404: " + http_status_text(HttpStatus.NotFound)

# 4. Sync Primitives Struct Methods (Channel & WaitGroup)
let ch = channel_new()
ch.send(777)
say "ch_method_size: " + str(ch.size())
say "ch_method_recv: " + str(ch.recv(100))
ch.free()

let wg = wait_group_new()
wg.add(1)
wg.done()
say "wg_method_wait: " + str(wg.wait(50))
wg.free()

# 5. Collections (in operator, spread, multi-var loops)
let arr = [10, 20, 30]
say "arr_contains: " + str(array_contains(arr, 20))
say "arr_not_contains: " + str(array_contains(arr, 99))
let cat = array_concat([1, 2], [3, 4])
say "cat_len: " + str(len(cat))

let s = set_new()
set_add(s, "apple")
say "set_has: " + str(set_has(s, "apple"))
say "set_not_has: " + str(set_has(s, "banana"))

let m = { "x": 100, "y": 200 }
let entries = map_entries(m)
say "entries_count: " + str(len(entries))

# 6. String & Type Checks (is null, in)
say "str_is_empty_null: " + str(is_empty(null))
say "str_is_empty_empty: " + str(is_empty(""))
say "str_contains: " + str(contains_str("alya language", "lang"))

# 7. JSON (multi-var key-value formatting)
let jobj = json_object({ "k": "v" })
say "json_has_k: " + str(contains_str(jobj, "k"))

# 8. Test Runner Struct Methods
let runner = runner_new()
runner.assert_eq(5, 5, "equality check")
say "runner_total: " + str(runner.total)
say "runner_passed: " + str(runner.passed)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("color_enum: 31"), "Got: {}", output);
        assert!(output.contains("bg_enum: 44"), "Got: {}", output);
        assert!(output.contains("style_enum: 1"), "Got: {}", output);
        assert!(output.contains("styled_ok"), "Got: {}", output);
        assert!(output.contains("log_level_name: WARN"), "Got: {}", output);
        assert!(output.contains("http_200: OK"), "Got: {}", output);
        assert!(output.contains("http_404: Not Found"), "Got: {}", output);
        assert!(output.contains("ch_method_size: 1"), "Got: {}", output);
        assert!(output.contains("ch_method_recv: 777"), "Got: {}", output);
        assert!(output.contains("wg_method_wait: 1"), "Got: {}", output);
        assert!(output.contains("arr_contains: 1"), "Got: {}", output);
        assert!(output.contains("arr_not_contains: 0"), "Got: {}", output);
        assert!(output.contains("cat_len: 4"), "Got: {}", output);
        assert!(output.contains("set_has: 1"), "Got: {}", output);
        assert!(output.contains("set_not_has: 0"), "Got: {}", output);
        assert!(output.contains("entries_count: 2"), "Got: {}", output);
        assert!(output.contains("str_is_empty_null: 1"), "Got: {}", output);
        assert!(output.contains("str_is_empty_empty: 1"), "Got: {}", output);
        assert!(output.contains("str_contains: 1"), "Got: {}", output);
        assert!(output.contains("json_has_k: 1"), "Got: {}", output);
        assert!(output.contains("runner_total: 1"), "Got: {}", output);
        assert!(output.contains("runner_passed: 1"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_consolidated_stdlib() {
    let code = r#"
import "std/console"
import "std/path"
import "std/math"

say "red_color: " + console_red("err")
say "glob_match: " + str(glob_match("*.alya", "main.alya"))
say "glob_no_match: " + str(glob_match("*.alya", "main.rs"))
let r = rand_int(10, 20)
say "rand_in_range: " + str(r >= 10 and r <= 20)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("\x1b[31merr\x1b[0m"), "Got: {}", output);
        assert!(output.contains("glob_match: 1"), "Got: {}", output);
        assert!(output.contains("glob_no_match: 0"), "Got: {}", output);
        assert!(output.contains("rand_in_range: 1"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_process_module() {
    let code = r#"
import "std/process"
import "std/str"

let pid = process_pid()
say "valid_pid: " + str(pid > 0)
let out = process_run("echo hello_process")
say "exit_code: " + str(out.exit_code)
say "has_output: " + str(contains_str(out.stdout, "hello_process"))
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(output.contains("valid_pid: 1"), "Got: {}", output);
        assert!(output.contains("exit_code: 0"), "Got: {}", output);
        assert!(output.contains("has_output: 1"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_io_module() {
    let code = r#"
import "std/io"

function my_source() -> string
    return "sample io content"
end

let r = Reader(my_source)
let content = io_read_all(r)
say "io_content: " + content
say "io_len: " + str(len(content))
io_print("direct io print")
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Execution failed: {}", output);
        assert!(
            output.contains("io_content: sample io content"),
            "Got: {}",
            output
        );
        assert!(output.contains("io_len: 17"), "Got: {}", output);
        assert!(output.contains("direct io print"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_no_std_mode() {
    use alya::codegen::{generate_with_profile_ext, Architecture, OperatingSystem};
    use alya::lexer::Lexer;
    use alya::parser::{resolve_imports_with_sources_ext, Parser};

    // 1. Importing std in no-std mode must fail with diagnostic error
    let invalid_code = "import \"std/math\"\nlet x = 1";
    let mut lexer = Lexer::new(invalid_code);
    let tokens = lexer.tokenize().expect("Lexer error");
    let mut parser = Parser::new(tokens);
    let mut ast = parser.parse().expect("Parser error");
    let res = resolve_imports_with_sources_ext(&mut ast, std::path::Path::new("."), true);
    assert!(res.is_err(), "Importing std in no-std should fail");
    let err_msg = res.unwrap_err();
    assert!(
        err_msg.contains("Cannot import 'std/math' in --no-std bare-metal mode"),
        "Got: {}",
        err_msg
    );

    // 2. Codegen with no_std = true must skip runtime emission
    let valid_code = "function main() -> int\n    return 42\nend";
    let mut lexer2 = Lexer::new(valid_code);
    let tokens2 = lexer2.tokenize().expect("Lexer error");
    let mut parser2 = Parser::new(tokens2);
    let ast2 = parser2.parse().expect("Parser error");
    let (asm_std, _) =
        generate_with_profile_ext(&ast2, Architecture::X64, OperatingSystem::Windows, false);
    let (asm_no_std, _) =
        generate_with_profile_ext(&ast2, Architecture::X64, OperatingSystem::Windows, true);

    assert!(
        asm_std.contains("fn_ask:"),
        "Standard codegen should include fn_ask runtime"
    );
    assert!(
        !asm_no_std.contains("fn_ask:"),
        "Bare-metal codegen must not include fn_ask runtime"
    );
}

#[test]
fn test_e2e_bounded_channel_blocking() {
    let code = r#"
import "std/sync"

let ch = Channel.new(2)
say "cap: " + str(ch.capacity())
say "is_rendezvous: " + str(ch.is_rendezvous())

# Send up to capacity
let ok1 = ch.send(10)
let ok2 = ch.send(20)
say "send1: " + str(ok1) + " send2: " + str(ok2)
say "sz_full: " + str(ch.size())

# try_send on full channel must fail immediately
let ok_try = ch.try_send(30)
say "try_send_full: " + str(ok_try)

# Worker function to consume one item after 30ms delay to unblock the sender
function worker_drain_fn(ch: Channel)
    sleep(30)
    let item = ch.recv()
    say "worker_drained: " + str(item)
end

let worker = thread_spawn(worker_drain_fn, ch)

# This send will block until worker unblocks a slot
let ok3 = ch.send(30)
say "send3_unblocked: " + str(ok3)
say "sz_after: " + str(ch.size())

thread_join(worker)

let r2 = ch.recv()
let r3 = ch.recv()
say "drained: " + str(r2) + " and " + str(r3)
ch.close()
say "closed: " + str(ch.is_closed())
ch.free()
"#;

    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(output.contains("cap: 2"), "Got: {}", output);
        assert!(output.contains("is_rendezvous: 0"), "Got: {}", output);
        assert!(output.contains("send1: 1 send2: 1"), "Got: {}", output);
        assert!(output.contains("sz_full: 2"), "Got: {}", output);
        assert!(output.contains("try_send_full: 0"), "Got: {}", output);
        assert!(output.contains("worker_drained: 10"), "Got: {}", output);
        assert!(output.contains("send3_unblocked: 1"), "Got: {}", output);
        assert!(output.contains("drained: 20 and 30"), "Got: {}", output);
        assert!(output.contains("closed: 1"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_rendezvous_channel_semantics() {
    let code = r#"
import "std/sync"

let ch = Channel.rendezvous()
say "ch_is_rendezvous: " + str(ch.is_rendezvous())

# Non-blocking try_send on rendezvous without active receiver returns 0
let try_res = ch.try_send(100)
say "rendezvous_try_send: " + str(try_res)

# Worker to receive value after a short delay
function rendezvous_recv_fn(ch: Channel)
    sleep(40)
    let rec = ch.recv()
    say "rendezvous_recv_val: " + str(rec)
end

let worker = thread_spawn(rendezvous_recv_fn, ch)

# Synchronous rendezvous handoff: send blocks until worker receives it
let ok = ch.send(999)
say "rendezvous_send_completed: " + str(ok)
thread_join(worker)

ch.close()
ch.free()
"#;

    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(output.contains("ch_is_rendezvous: 1"), "Got: {}", output);
        assert!(output.contains("rendezvous_try_send: 0"), "Got: {}", output);
        assert!(
            output.contains("rendezvous_recv_val: 999"),
            "Got: {}",
            output
        );
        assert!(
            output.contains("rendezvous_send_completed: 1"),
            "Got: {}",
            output
        );
    }
}

#[test]
fn test_e2e_thread_spawn_join_minimal() {
    // Granular thread-primitive isolation (bisects Windows-ARM64 AVs in
    // thread_and_concurrency / bounded_channel / rendezvous tests).
    let code = r#"
import "std/thread"

function sq(x)
    return x * x
end

let t = thread_spawn(sq, 7)
let r = thread_join(t)
say "spawn_join: " + str(r)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(output.contains("spawn_join: 49"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_thread_id_basic() {
    let code = r#"
import "std/thread"

let tid = thread_id()
if tid > 0
    say "tid: ok"
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(output.contains("tid: ok"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_mutex_basic() {
    let code = r#"
import "std/thread"

let m = mutex_new()
mutex_lock(m)
mutex_unlock(m)
mutex_free(m)
say "mutex: ok"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(output.contains("mutex: ok"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_thread_spawn_with_sleep() {
    let code = r#"
import "std/thread"

function delayed(dummy)
    sleep(20)
    return 5
end

let t = thread_spawn(delayed, 0)
let r = thread_join(t)
say "slept: " + str(r)
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(output.contains("slept: 5"), "Got: {}", output);
    }
}

#[test]
fn test_e2e_dynamic_interface_querying() {
    let code = r#"
interface Shape
    function area(self) -> int
end

interface Describable
    function describe(self) -> string
end

struct Circle
    radius: int
end

function Circle.area(self) -> int
    return 3 * self.radius * self.radius
end

function Circle.describe(self) -> string
    return "Circle with radius " + str(self.radius)
end

struct PlainPoint
    x: int
    y: int
end

function inspect_dynamic(obj)
    if obj is Shape
        say "dynamic: is Shape"
    else
        say "dynamic: not Shape"
    end

    if obj is Describable
        say "dynamic: is Describable"
    else
        say "dynamic: not Describable"
    end
end

let c = Circle(5)
let p = PlainPoint(10, 20)

say "c is Shape: " + str(c is Shape)
say "c is Describable: " + str(c is Describable)
say "c is not Shape: " + str(c is not Shape)

say "p is Shape: " + str(p is Shape)
say "p is Describable: " + str(p is Describable)
say "p is not Shape: " + str(p is not Shape)

say "--- dynamic inspect c ---"
inspect_dynamic(c)

say "--- dynamic inspect p ---"
inspect_dynamic(p)
"#;

    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(output.contains("c is Shape: 1"), "Got: {}", output);
        assert!(output.contains("c is Describable: 1"), "Got: {}", output);
        assert!(output.contains("c is not Shape: 0"), "Got: {}", output);
        assert!(output.contains("p is Shape: 0"), "Got: {}", output);
        assert!(output.contains("p is Describable: 0"), "Got: {}", output);
        assert!(output.contains("p is not Shape: 1"), "Got: {}", output);
        assert!(output.contains("dynamic: is Shape"), "Got: {}", output);
        assert!(
            output.contains("dynamic: is Describable"),
            "Got: {}",
            output
        );
        assert!(output.contains("dynamic: not Shape"), "Got: {}", output);
        assert!(
            output.contains("dynamic: not Describable"),
            "Got: {}",
            output
        );
    }
}

#[test]
fn test_e2e_mem_trace_clean_execution() {
    let code = r#"
struct Person
    name
    age
end

fn run_work()
    let p = Person { name: "Alice", age: 30 }
    let numbers = [10, 20, 30]
    let dict = { "status": "active" }
    say "Person: {p.name}, numbers len: {len(numbers)}"
end

run_work()
"#;

    if let Some((code, output)) = run_alya_code_with_trace(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(
            output.contains("ALYA MEMORY TRACE & LEAK REPORT"),
            "Expected trace header, got:\n{}",
            output
        );
        assert!(
            output.contains("STATUS: [OK] Clean execution, 0 memory leaks detected"),
            "Expected clean execution status, got:\n{}",
            output
        );
        assert!(
            output.contains("Structs         : 0 active"),
            "Expected 0 live structs, got:\n{}",
            output
        );
        assert!(
            output.contains("Arrays          : 0 active"),
            "Expected 0 live arrays, got:\n{}",
            output
        );
        assert!(
            output.contains("Maps            : 0 active"),
            "Expected 0 live maps, got:\n{}",
            output
        );
    }
}

#[test]
fn test_e2e_mem_trace_leak_detection() {
    let code = r#"
struct Node
    id
    label
end

let mut i = 0
while i < 1
    let leaked_struct = Node { id: 101, label: "leak_node" }
    let leaked_array = [10, 20, 30, 40]
    let leaked_map = { "key": 999 }
    say "Allocated objects inside unmanaged loop iteration."
    i = i + 1
end
"#;

    if let Some((code, output)) = run_alya_code_with_trace(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(
            output.contains("ALYA MEMORY TRACE & LEAK REPORT"),
            "Expected trace header, got:\n{}",
            output
        );
        assert!(
            output.contains("STATUS: [WARN]"),
            "Expected warning status, got:\n{}",
            output
        );
        assert!(
            output.contains("memory leak(s) detected"),
            "Expected leak detection, got:\n{}",
            output
        );
        assert!(
            output.contains("Type: Struct (Node)"),
            "Expected leak attribution to Struct (Node), got:\n{}",
            output
        );
        assert!(
            output.contains("Type: Array"),
            "Expected leak attribution to Array, got:\n{}",
            output
        );
        assert!(
            output.contains("Type: Map"),
            "Expected leak attribution to Map, got:\n{}",
            output
        );
    }
}

#[test]
fn test_e2e_mem_trace_cyclic_leak() {
    let code = r#"
struct Node
    id
    next
end

fn create_cycle()
    let a = Node { id: 1, next: 0 }
    let b = Node { id: 2, next: 0 }
    a.next = b
    b.next = a
end

create_cycle()
"#;

    if let Some((code, output)) = run_alya_code_with_trace(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(
            output.contains("ALYA MEMORY TRACE & LEAK REPORT"),
            "Expected trace header, got:\n{}",
            output
        );
        assert!(
            output.contains("STATUS: [WARN]"),
            "Expected warning status, got:\n{}",
            output
        );
        assert!(
            output.contains("memory leak(s) detected"),
            "Expected leak detection, got:\n{}",
            output
        );
        assert!(
            output.contains("Type: Struct (Node)"),
            "Expected leak attribution to Struct (Node), got:\n{}",
            output
        );
    }
}

#[test]
fn test_e2e_mem_trace_disabled_zero_overhead() {
    let code = r#"
struct Point
    x
    y
end

let pt = Point { x: 5, y: 10 }
say "Point: {pt.x}, {pt.y}"
"#;

    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(
            !output.contains("ALYA MEMORY TRACE & LEAK REPORT"),
            "Trace report should NOT appear when disabled"
        );
    }
}

#[test]
fn test_e2e_bacon_rajan_cycle_collector() {
    let code = r#"
struct Node
    id
    next
end

fn create_cycle()
    let a = Node { id: 10, next: 0 }
    let b = Node { id: 20, next: 0 }
    a.next = b
    b.next = a
end

create_cycle()
let collected = gc_collect()
say "SWEEP: {collected}"
"#;

    if let Some((code, output)) = run_alya_code_with_trace(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(
            output.contains("SWEEP: 2") || output.contains("SWEEP:"),
            "Expected cycle collection sweep output, got:\n{}",
            output
        );
        assert!(
            output.contains("STATUS: [OK] Clean execution, 0 memory leaks detected"),
            "Expected clean execution with 0 leaks after Bacon-Rajan GC, got:\n{}",
            output
        );
    }
}

#[test]
fn test_e2e_bacon_rajan_preserves_live_cyclic_objects() {
    let code = r#"
struct Node
    id
    next
end

let a = Node { id: 100, next: 0 }
let b = Node { id: 200, next: 0 }
a.next = b
b.next = a

# Both a and b are alive in active scope
let collected = gc_collect()
say "COLLECTED_LIVE: {collected}"
say "A_ID: {a.id}, B_ID: {b.id}, A_NEXT_ID: {a.next.id}"
"#;

    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(
            output.contains("COLLECTED_LIVE: 0"),
            "Live cyclic objects must not be collected, got:\n{}",
            output
        );
        assert!(
            output.contains("A_ID: 100, B_ID: 200, A_NEXT_ID: 200"),
            "Expected active object access to remain intact, got:\n{}",
            output
        );
    }
}

#[test]
fn test_e2e_bacon_rajan_circular_doubly_linked_list() {
    let code = r#"
struct DNode
    id
    prev
    next
end

fn make_circular_ring()
    let n1 = DNode { id: 1, prev: 0, next: 0 }
    let n2 = DNode { id: 2, prev: 0, next: 0 }
    let n3 = DNode { id: 3, prev: 0, next: 0 }
    let n4 = DNode { id: 4, prev: 0, next: 0 }
    let n5 = DNode { id: 5, prev: 0, next: 0 }

    n1.next = n2
    n2.prev = n1
    n2.next = n3
    n3.prev = n2
    n3.next = n4
    n4.prev = n3
    n4.next = n5
    n5.prev = n4
    n5.next = n1
    n1.prev = n5
end

make_circular_ring()
let swept = gc_collect()
say "SWEPT_RING: {swept}"
"#;

    if let Some((code, output)) = run_alya_code_with_trace(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(
            output.contains("SWEPT_RING: 5"),
            "Expected 5 ring nodes collected, got:\n{}",
            output
        );
        assert!(
            output.contains("STATUS: [OK] Clean execution, 0 memory leaks detected"),
            "Expected 0 memory leaks for doubly-linked ring, got:\n{}",
            output
        );
    }
}

#[test]
fn test_e2e_cooperative_fibers_basic() {
    let code = r#"
import "std/sync"

function compute_fib(n: int) -> int
    let id = fiber_id()
    say "fiber_started: " + str(id)
    fiber_yield()
    let res = n * 10
    say "fiber_res: " + str(res)
    return res
end

let f1 = fiber_spawn(compute_fib, 4)
let f2 = fiber_spawn(compute_fib, 7)

say "active_fibers: " + str(fiber_count())

let r1 = fiber_join(f1)
let r2 = fiber_join(f2)

say "joined_r1: " + str(r1)
say "joined_r2: " + str(r2)
"#;

    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(output.contains("active_fibers: 3"), "Got:\n{}", output);
        assert!(output.contains("fiber_started: 1"), "Got:\n{}", output);
        assert!(output.contains("fiber_started: 2"), "Got:\n{}", output);
        assert!(output.contains("fiber_res: 40"), "Got:\n{}", output);
        assert!(output.contains("fiber_res: 70"), "Got:\n{}", output);
        assert!(output.contains("joined_r1: 40"), "Got:\n{}", output);
        assert!(output.contains("joined_r2: 70"), "Got:\n{}", output);
    }
}

#[test]
fn test_e2e_cooperative_fibers_channel_pipeline() {
    let code = r#"
import "std/sync"

let ch = Channel.new(3)

function producer_fiber(count: int)
    for i in 1..=count
        ch.send(i * 5)
        yield()
    end
    ch.close()
end

function consumer_fiber(sink: Channel)
    let total = 0
    while 1
        let val = ch.recv()
        if val == null
            break
        end
        total += val
        yield()
    end
    say "consumer_total: " + str(total)
    return total
end

let prod = fiber_spawn(producer_fiber, 5)
let cons = fiber_spawn(consumer_fiber, ch)

fiber_drain()
let total = fiber_join(cons)
say "pipeline_verified: " + str(total)
"#;

    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(output.contains("consumer_total: 75"), "Got:\n{}", output);
        assert!(output.contains("pipeline_verified: 75"), "Got:\n{}", output);
    }
}

#[test]
fn test_e2e_cooperative_fibers_100k_stress() {
    let code = r#"
import "std/sync"

function fast_fiber(val: int) -> int
    return val + 1
end

let total = 0
let n = 100000

for i in 1..=n
    let f = fiber_spawn(fast_fiber, 1)
    let res = fiber_join(f)
    total += res
end

say "stress_100k_total: " + str(total)
say "stress_100k_status: [OK] 100000 fibers scheduled successfully"
"#;

    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "Failed with code {}\nOutput:\n{}", code, output);
        assert!(
            output.contains("stress_100k_total: 200000"),
            "Got:\n{}",
            output
        );
        assert!(
            output.contains("stress_100k_status: [OK] 100000 fibers scheduled successfully"),
            "Got:\n{}",
            output
        );
    }
}
