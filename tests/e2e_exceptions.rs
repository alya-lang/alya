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
fn test_e2e_throw_concurrent_threads_keep_catches() {
    // alya-lang/alya#65: catch state must be per-thread; concurrent
    // throws must not steal each other's catch. Workers send idx+1
    // (never bare 0: int 0 shares null's zero word, so an `is null`
    // emptiness check cannot tell them apart); the main thread asserts
    // the exact count and sum, proving every catch fired exactly once.
    let code = r#"
import "std/thread"

function worker(ctx)
    let ch, idx = ctx
    try
        throw "boom"
    catch
        channel_send(ch, idx + 1)
    end
end

function main()
    let ch = channel_new(128)
    let threads = []
    let i = 0
    while i < 64
        threads.push(thread_spawn(worker, (ch, i)))
        i += 1
    end
    let got = 0
    let total = 0
    while got < 64
        let m = channel_recv(ch, 5000)
        if m is null
            break
        end
        got += 1
        total += m
    end
    for th in threads
        thread_join(th)
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
