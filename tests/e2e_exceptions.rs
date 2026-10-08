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

#[test]
fn test_e2e_throw_struct_caught_keeps_string_field() {
    // alya-lang/alya#126: `.message` on a catch binding must read the
    // throw-recorded struct text, not the struct pointer (which printed
    // empty); int fields and plain-string throws are unaffected.
    let code = r#"
struct CaughtFields
    message: string
    code: int
end
try
    throw CaughtFields { message: "hello", code: 42 }
catch err
    say err.code
    say err.message
end
try
    throw "boom"
catch err
    say err.message
end
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "42\nhello\nboom\n");
    }
}

#[test]
fn test_e2e_throw_uncaught_in_worker_prints_message() {
    // Fatal path (printf + exit) on a pthread-created thread.
    let code = r#"
import "std/thread"

function wdiag(idx)
    throw "worker-boom"
end

function main()
    let t = thread_spawn(wdiag, 0)
    thread_join(t)
    say "joined"
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_ne!(code, 0, "expected fatal exit, got output: {}", output);
        assert!(
            output.contains("Runtime error: worker-boom"),
            "Got code {} output: {}",
            code,
            output
        );
    }
}

#[test]
fn test_e2e_try_no_throw_in_worker_thread() {
    // try/begin/end in workers without throwing.
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
fn test_e2e_throw_caught_in_worker_thread() {
    // alya-lang/alya#68: worker-thread caught exception must preserve
    // callee-saved registers so returning to the OS thread trampoline
    // (_pthread_start) does not signal.
    let code = r#"
import "std/thread"

function wdiag(idx)
    let out = -999
    try
        throw "boom"
    catch
        out = idx + 1
    end
    return out
end

function main()
    let t = thread_spawn(wdiag, 0)
    say thread_join(t)
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0, "output was: {}", output);
        assert_eq!(output, "1\n");
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
    // (alya-lang/alya#67) and must not gate this test.
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

#[test]
fn test_e2e_null_field_store_trapped() {
    // alya-lang/alya#74: a field store through a null base must raise a
    // catchable runtime error instead of segfaulting.
    let code = r#"
struct Box
    val: int
end

function main()
    let b: any = null
    try
        b.val = 42
        say "UNREACHABLE"
    catch err
        say "caught: " + err
    end
    let ok = Box { val: 7 }
    ok.val = 8
    say ok.val
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "caught: field access on null\n8\n");
    }
}

#[test]
fn test_e2e_null_field_load_trapped() {
    // alya-lang/alya#74, load side: same trap, valid access unaffected.
    let code = r#"
struct Box
    val: int
end

function main()
    let b: any = null
    try
        say b.val
        say "UNREACHABLE"
    catch err
        say "caught: " + err
    end
end

main()
"#;
    if let Some((code, output)) = run_alya_code_full(code) {
        assert_eq!(code, 0);
        assert_eq!(output, "caught: field access on null\n");
    }
}
