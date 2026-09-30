mod common;
use common::*;

#[test]
fn test_e2e_div_by_zero_protection() {
    let code = r#"
let a = 50
let b = 0
say a / b
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_ne!(code, 0);
        assert!(output.contains("Runtime error: division by zero"));
    }
}

#[test]
fn test_e2e_modulo_by_zero_protection() {
    let code = r#"
say 100 % 0
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_ne!(code, 0);
        assert!(output.contains("Runtime error: division by zero"));
    }
}

#[test]
fn test_e2e_try_catch_basic() {
    let code = r#"
say "Before try"
try
    say "Inside try before error"
    let x = 10 / 0
    say "Should not print"
catch
    say "Caught error successfully"
end
say "After try"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "Before try\nInside try before error\nCaught error successfully\nAfter try\n"
        );
    }
}

#[test]
fn test_e2e_try_catch_with_err_var() {
    let code = r#"
try
    let a = 100 % 0
catch err
    say "Caught: " + err
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "Caught: division by zero\n");
    }
}

#[test]
fn test_e2e_try_catch_no_error() {
    let code = r#"
try
    let x = 10 / 2
    say x
catch
    say "Should not print"
end
say "Done"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "5\nDone\n");
    }
}

#[test]
fn test_e2e_array_bounds_catch() {
    let code = r#"
let arr = [1, 2, 3]
try
    let x = arr[5]
    say x
catch err
    say "caught: " + err
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "caught: index out of bounds\n");
    }
}

#[test]
fn test_e2e_throw_custom_string() {
    let code = r#"
say "Start"
try
    say "Inside try"
    throw "user validation failed"
    say "Unreachable"
catch err
    say "Caught: " + err
end
say "After"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "Start\nInside try\nCaught: user validation failed\nAfter\n"
        );
    }
}

#[test]
fn test_e2e_throw_number() {
    let code = r#"
try
    throw 404
catch err
    say "Status: " + err
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "Status: 404\n");
    }
}

#[test]
fn test_e2e_throw_struct_uncaught_prints_message() {
    let code = r#"
struct SocketError
    message: string
end
throw SocketError { message: "connection refused" }
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_ne!(code, 0);
        assert!(output.contains("Runtime error: connection refused"));
    }
}

#[test]
fn test_e2e_throw_struct_caught_keeps_value() {
    let code = r#"
struct SocketError
    message: string
end
try
    throw SocketError { message: "connection refused" }
catch err
    say "caught"
    say err is not null
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "caught\n1\n");
    }
}

// TEMP-DIAG (remove after macOS ARM64 64-thread crash is bisected):
// count x throw matrix to isolate the crashing axis.
fn diag_prog(n: i32, with_throw: bool) -> String {
    let body = if with_throw {
        "    let out = -999\n    try\n        throw \"boom\"\n    catch\n        out = idx + 1\n    end\n    return out\n"
    } else {
        "    return idx + 1\n"
    };
    format!(
        "import \"std/thread\"\n\nfunction wdiag(idx)\n{}end\n\nfunction main()\n    let threads = []\n    let i = 0\n    while i < {}\n        threads.push(thread_spawn(wdiag, i))\n        i += 1\n    end\n    let total = 0\n    for th in threads\n        total += thread_join(th)\n    end\n    say total\nend\n\nmain()\n",
        body, n
    )
}

fn diag_expect(n: i32) -> String {
    format!("{}\n", n * (n + 1) / 2)
}

#[test]
fn test_diag_threads_04_plain() {
    if let Some((code, output)) = run_alya_code_full(&diag_prog(4, false)) {
        assert_eq!(code, 0, "output was: {}", output);
        assert_eq!(output, diag_expect(4));
    }
}

#[test]
fn test_diag_threads_04_throw() {
    if let Some((code, output)) = run_alya_code_full(&diag_prog(4, true)) {
        assert_eq!(code, 0, "output was: {}", output);
        assert_eq!(output, diag_expect(4));
    }
}

#[test]
fn test_diag_threads_04_try_no_throw() {
    // TEMP-DIAG: try/begin/end without throw — isolates once+block+
    // begin/end from the throw/dispatch path on macOS ARM64.
    let code = r#"
import "std/thread"

function wdiag(idx)
    let out = idx + 1
    try
        out = out + 0
    catch
        out = -999
    end
    return out
end

function main()
    let threads = []
    let i = 0
    while i < 4
        threads.push(thread_spawn(wdiag, i))
        i += 1
    end
    let total = 0
    for th in threads
        total += thread_join(th)
    end
    say total
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "output was: {}", output);
        assert_eq!(output, "10\n");
    }
}

#[test]
fn test_diag_threads_64_plain() {
    if let Some((code, output)) = run_alya_code_full(&diag_prog(64, false)) {
        assert_eq!(code, 0, "output was: {}", output);
        assert_eq!(output, diag_expect(64));
    }
}

#[test]
fn test_diag_threads_64_throw() {
    if let Some((code, output)) = run_alya_code_full(&diag_prog(64, true)) {
        assert_eq!(code, 0, "output was: {}", output);
        assert_eq!(output, diag_expect(64));
    }
}

#[test]
fn test_e2e_throw_concurrent_threads_keep_catches() {
    // alya-lang/alya#65: catch state must be per-thread; concurrent
    // throws must not steal each other's catch. Each worker throws
    // and catches locally, reporting via its join value (idx+1, never
    // bare 0: int 0 shares null's zero word). The main thread asserts
    // the exact count and sum, proving every catch fired exactly once
    // in its own thread. Join values (not channels) carry results:
    // channel_send under heavy contention has its own known races
    // and must not gate this test.
    let code = r#"
import "std/thread"

function worker(idx)
    let out = -999
    try
        throw "boom"
    catch
        out = idx + 1
    end
    return out
end

function main()
    let threads = []
    let i = 0
    while i < 64
        threads.push(thread_spawn(worker, i))
        i += 1
    end
    let got = 0
    let total = 0
    for th in threads
        total += thread_join(th)
        got += 1
    end
    say got
    say total
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "64\n2080\n");
    }
}

#[test]
fn test_e2e_catch_parentheses() {
    let code = r#"
try
    throw "parens test"
catch (e)
    say "Handled: " + e
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "Handled: parens test\n");
    }
}

#[test]
fn test_e2e_finally_on_success() {
    let code = r#"
try
    say "Executing task"
catch err
    say "Caught: " + err
finally
    say "Cleaned up resources"
end
say "Finished"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "Executing task\nCleaned up resources\nFinished\n");
    }
}

#[test]
fn test_e2e_finally_on_error() {
    let code = r#"
try
    say "Starting"
    throw "database timeout"
catch err
    say "Handled error: " + err
finally
    say "Closing connection"
end
say "Done"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "Starting\nHandled error: database timeout\nClosing connection\nDone\n"
        );
    }
}

#[test]
fn test_e2e_try_finally_without_catch() {
    let code = r#"
try
    say "Outer try"
    try
        say "Inner try"
        throw "something broke"
    finally
        say "Inner cleanup"
    end
catch err
    say "Outer caught: " + err
end
say "All complete"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "Outer try\nInner try\nInner cleanup\nOuter caught: something broke\nAll complete\n"
        );
    }
}

#[test]
fn test_e2e_rethrow() {
    let code = r#"
try
    try
        throw "first error"
    catch err
        say "Log: " + err
        throw
    end
catch e
    say "Rethrown: " + e
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "Log: first error\nRethrown: first error\n");
    }
}

#[test]
fn test_e2e_unhandled_throw() {
    let code = r#"
say "Starting program"
throw "fatal unhandled crash"
say "Never reaches here"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_ne!(code, 0);
        assert!(output.contains("Runtime error: fatal unhandled crash"));
    }
}

#[test]
fn test_e2e_finally_with_catch_and_rethrow() {
    let code = r#"
try
    try
        throw "inner fail"
    catch err
        say "Catch: " + err
        throw
    finally
        say "Inner finally"
    end
catch e
    say "Outer: " + e
end
say "Done"
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(
            output,
            "Catch: inner fail\nInner finally\nOuter: inner fail\nDone\n"
        );
    }
}
