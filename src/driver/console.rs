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
