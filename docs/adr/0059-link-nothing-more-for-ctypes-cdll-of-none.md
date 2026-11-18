# ADR-0059: Link nothing more for `ctypes.CDLL(None)`

## Status

Accepted

## Date

2026-11-18

## Context

A `ctypes` declaration names its library by a string literal, which the
build links (FFI-01, ADR-0047). The file names of the C library and the C
maths library differ between systems, `libc.so.6` and `libm.so.6` on Linux
and `libc.dylib` and `libm.dylib` on macOS, so a program that calls `abs`
or `cbrt` through a literal runs on one system only. On macOS, CPython
raises `OSError` when it loads `libc.so.6`, where TyPy stops the build.

CPython's `ctypes.CDLL(None)` loads the process itself, through
`dlopen(NULL)`, and finds every function the process has loaded. The
CPython 3.12 to 3.15 oracles find the functions of both libraries through
it on Linux, checked on 2026-11-18, and macOS's `libSystem` holds both.
Every TyPy program already links the C library, and the maths library
before the runtime archive (ADR-0006).

## Options considered

1. **Keep string literals only.** Tests and programs that need only C's own
   functions are written twice, once for each system.
2. **Treat `CDLL(None)` as the C and maths libraries every program links,**
   with nothing added to the link.
3. **Resolve the functions of `CDLL(None)` when the program starts,** with
   `dlsym`, as CPython does. A second call path, against ADR-0047's direct
   calls.

## Decision

Option 2. `CDLL(None)` and `cdll.LoadLibrary(None)` declare a library with
no name; its functions are called directly, and the link adds no library
for it. A function neither library defines is E0702 when the program is
linked, as a function missing from a named library is.

## Consequences

- A program that calls only functions of the C and maths libraries builds
  and runs alike on Linux and macOS, and the conformance tests of FFI-02
  and FFI-03 call them through `CDLL(None)`.
- A function another library of the process defines, which CPython's
  `CDLL(None)` would find, is not found; such a program names its library.

## Related requirements

FFI-01, FFI-14, NFR-17, ADR-0006, ADR-0047.
