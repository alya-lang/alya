//! Console initialization and terminal configuration.

#[cfg(windows)]
pub fn init_console() {
    type Handle = *mut std::ffi::c_void;
    type Dword = u32;
    type Bool = i32;

    const STD_OUTPUT_HANDLE: Dword = -11i32 as Dword;
    const STD_ERROR_HANDLE: Dword = -12i32 as Dword;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: Dword = 0x0004;
    const CP_UTF8: u32 = 65001;

    extern "system" {
        fn GetStdHandle(nStdHandle: Dword) -> Handle;
        fn GetConsoleMode(hConsoleHandle: Handle, lpMode: *mut Dword) -> Bool;
        fn SetConsoleMode(hConsoleHandle: Handle, dwMode: Dword) -> Bool;
        fn SetConsoleOutputCP(wCodePageID: u32) -> Bool;
        fn SetConsoleCP(wCodePageID: u32) -> Bool;
    }

    unsafe {
        let _ = SetConsoleOutputCP(CP_UTF8);
        let _ = SetConsoleCP(CP_UTF8);

        for handle_id in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let handle = GetStdHandle(handle_id);
            if !handle.is_null() && handle as isize != -1 {
                let mut mode: Dword = 0;
                if GetConsoleMode(handle, &mut mode) != 0 {
                    let _ = SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
                }
            }
        }
    }
}

#[cfg(not(windows))]
pub fn init_console() {
    // Linux and macOS support VT escape sequences and UTF-8 natively.
}

/// Computes visible display width of a string (ignoring ANSI escape codes and accounting for wide characters/emojis).
pub fn visible_width(s: &str) -> usize {
    let mut width = 0;
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else if ch == '⚡' || (ch >= '\u{1F300}' && ch <= '\u{1FAFF}') {
            width += 2;
        } else {
            width += 1;
        }
    }
    width
}

/// Prints a row inside a box with left and right `║` borders, padding spaces automatically to `inner_width`.
pub fn print_box_row(content: &str, inner_width: usize) {
    let vis = visible_width(content);
    let pad = if inner_width > vis {
        inner_width - vis
    } else {
        0
    };
    println!(
        "\x1b[1;36m║\x1b[0m{}{}\x1b[1;36m║\x1b[0m",
        content,
        " ".repeat(pad)
    );
}
