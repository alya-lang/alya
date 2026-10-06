// SEH-unwind-through-Alya-frames probe (alya-lang/alya#109).
//
// Windows-only (wired via `c-sources-windows`): the OS unwinder cannot
// walk Alya frames without .pdata/.xdata. Two checks:
//   1. `probe_has_pdata` — RtlLookupFunctionEntry finds the CALLER
//      (an Alya frame) in .pdata. NOTE: ImageBase must be a real
//      pointer; this build's ntdll faults on NULL there.
//   2. `run_unwind_test` — setjmp/longjmp (RtlUnwindEx internally)
//      unwinds THROUGH the exported Alya frame `alya_probe_raiser`.
//      Without unwind info this faults (the issue's crash); with it,
//      control lands back here with value 42.
#include <windows.h>
#include <setjmp.h>

extern long long alya_probe_raiser(void);

static jmp_buf jb;

long long probe_do_raise(void) {
    longjmp(jb, 42);
    return -1;
}

long long probe_has_pdata(void) {
    void *ra = __builtin_return_address(0);
    DWORD64 base = 0;
    return RtlLookupFunctionEntry((DWORD64)ra, &base, NULL) != NULL ? 1 : 0;
}

long long run_unwind_test(void) {
    if (setjmp(jb) == 0) {
        alya_probe_raiser();
        return -1;
    }
    return 42;
}
