# ADR-0032: Export a module as a C library described by its own annotations

## Status

Accepted

## Date

2026-11-15

## Context

TyPy 0.5 builds a program's code as a static or a shared library (CLI-09)
and exports functions with C symbols and a C header (FFI-05). The aim is
that Python programmers write native libraries in Python, and that C, C++,
Rust and Python programs call them as cheaply as a C library.

Two constraints shape the design:

- Every TyPy program is standard Python that CPython runs with the same
  result (ADR-0002, STD-07). The exported functions must be ordinary
  functions that CPython can import and call, and a program that uses the
  library must run on CPython too.
- TyPy's object layouts, exception state and type objects are not a stable
  ABI before 1.0 (OI-23). A Python object must not cross the boundary of a
  library, or every caller would depend on that layout.

FFI-05 as written lists each exported function and its C types under
`[export]` in `typy.toml`. That splits a function's interface between its
source and a configuration file, which a type checker and an editor cannot
see.

## Options considered

1. **A table in `typy.toml`**, as FFI-05 says today: names and C types
   beside the source.
2. **The module's own annotations.** The functions in `__all__` of the
   module given to `typy build --lib` are the exports. Their annotations
   give the C types: `int`, `float` and `bool` map to `int64_t`, `double`
   and `bool`, and the `ctypes` types of FFI-03, such as `c_int32` or
   `c_char_p`, give an exact C type. Any other type is refused for an
   export.
3. **A decorator of TyPy's own**, such as `@export`. CPython could not run
   it without a module of TyPy's, which STD-07 forbids.

For how other programs call the library:

1. **The C header only.** C and C++ call it; Python cannot without writing
   its own bindings.
2. **The C header, a `.pyi` stub and a `ctypes` binding module.** The
   build writes all three. The binding is standard Python that loads the
   library with `ctypes.CDLL` and sets each function's `argtypes` and
   `restype`. CPython runs it as it is, and TyPy compiles its calls into
   direct C calls (FFI-01, FFI-02), so one source serves both.
3. **A CPython extension module.** It would tie the library to one CPython
   version's C API and to TyPy's object layout, which OI-23 has not fixed.

## Decision

TyPy will use option 2 for the exports and option 2 for the callers.

- `typy build --lib static` or `--lib shared` on a module exports the
  functions its `__all__` lists, each under its own name as an unmangled C
  symbol. A listed name that is not a function, or a parameter or result
  whose type has no C type, is a diagnostic that names it.
- `int`, `float` and `bool` map to `int64_t`, `double` and `bool`; an
  `int` outside `int64_t` is `OverflowError` on the TyPy side of the call.
  A `ctypes` type gives its own C type, with the conversions FFI-03
  defines. `None` as the result type is `void`.
- An exception that would leave an exported function aborts the program
  with the exception printed (EXC-05), since C has no exceptions.
- The build writes three files beside the library: a C header, a `.pyi`
  stub of the exported functions with the module's own annotations, and a
  `ctypes` binding module that loads the library and declares each
  function. A program imports the binding and runs the same under CPython
  and TyPy.
- The `[export]` table of `typy.toml` is not added; FFI-05 is reworded to
  name `__all__` and the annotations.

## Consequences

- A library's interface is written once, in Python, where type checkers,
  editors and readers see it.
- No Python object crosses the library's boundary, so a library built by
  one TyPy release links with programs built by another, and OI-23 does not
  block libraries.
- Exports are limited to C types: a function that takes or returns a
  `str`, a `list` or an instance needs a wrapper with C types, as a C
  library would. Structures follow with `ctypes.Structure` (FFI-03).
- CPython callers pay for `ctypes` conversions; TyPy callers do not.

## Related requirements

CLI-09, FFI-01, FFI-02, FFI-03, FFI-04, FFI-05, FFI-07, FFI-08, EXC-05, STD-07,
OI-23, ADR-0002, ADR-0016.
