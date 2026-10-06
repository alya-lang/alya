// C++ dispatch through Alya frames (alya-lang/alya#109).
//
// Windows-only (wired via `c-sources-windows`, statically linked so the
// fixture is self-contained): models the Intel GPU driver scenario with
// full control — a C++ exception is thrown above an exported Alya frame
// and caught below it:
//
//   fn_main (Alya) -> cpp_entry (C++, try/catch) -> alya_mid (Alya,
//   @export) -> cpp_thrower (C++, throws int 777).
//
// If the catch fires with 777, OS/C++ dispatch survived the Alya frame.
// Regression for the vectored-handler rsp clobber: the crash handler
// must leave rsp pristine on its CONTINUE_SEARCH path, or dispatch
// reads every frame slot shifted and jumps wild.
#include <stdio.h>

extern "C" long long alya_mid(void);

extern "C" void cpp_thrower(void) {
    throw 777;
}

extern "C" long long cpp_entry(void) {
    try {
        return alya_mid();
    } catch (int x) {
        return (long long)x * 1000 + 7;
    } catch (...) {
        return -99;
    }
}
