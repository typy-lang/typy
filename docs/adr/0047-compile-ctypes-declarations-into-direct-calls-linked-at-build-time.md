# ADR-0047: Compile ctypes declarations into direct calls linked at build time

## Status

Accepted

## Date

2026-11-17

## Context

ADR-0003 decides that a program calls C through the standard `ctypes`
module, so it runs under CPython too, and that TyPy compiles the
declarations into direct calls. FFI-01 to FFI-04 ask for that in 0.5: a
library named by a string literal is linked when the program is built, a
function declared by its `argtypes` and `restype` at module level is
called with the conversions `ctypes` defines, and `typy.toml` says whether
a library is linked statically or dynamically. This record covers
libraries, functions and the scalar types; pointers and structures follow
in records of their own.

CPython 3.12 to 3.15 convert alike, measured on 2026-11-17 through
`libc.so.6` and `libm.so.6`:

| Type | Argument | Result |
| --- | --- | --- |
| `c_int8` to `c_uint64`, `c_int`, `c_long`, `c_size_t`, `c_ssize_t` | An `int` or a `bool`, wrapped to the type's width | The value, signed or unsigned as the type is |
| `c_bool` | Any value, by its truth | `True` when the low byte is not zero |
| `c_float`, `c_double` | An `int`, a `float` or a `bool`; an `int` too large for a `float` raises `ctypes.ArgumentError` | A `float`; `c_float` rounds through 32 bits |
| `c_char_p` | `bytes`, whose own buffer C reads, or `None` for null | A copy of the C string as `bytes`, or `None` for null |
| `c_void_p` | An `int` wrapped to 64 bits, `bytes`, or `None` for null | An `int`, or `None` for null |

Any other argument raises `ctypes.ArgumentError`, whose text 3.15 words
differently for `c_char_p`. CPython loads a library when `CDLL` runs and
finds a function when the program first names it, so a missing library is
`OSError` and a missing function `AttributeError`, at run time.

## Options considered

1. **A `ctypes` of the runtime's own**: `dlopen`, `dlsym` and a call made
   at run time from the declared types, as CPython's `ctypes` does with
   `libffi`. The program behaves as CPython's does in every case, and
   every call pays for the dynamic call and the checks.
2. **Declarations compiled with the program.** The checker reads each
   library and each function's declaration from the module, types each
   call from it, and code generation calls the function directly with
   the conversions inline; the library is linked when the program is
   built.

## Decision

TyPy uses option 2, as ADR-0003 foresees.

- **Libraries.** `NAME = ctypes.CDLL("library")` and
  `NAME = ctypes.cdll.LoadLibrary("library")`, at module level with a
  string literal, under any name `import ctypes`, `import ctypes as C` or
  `from ctypes import CDLL, cdll` gives, declare the library, which the
  build links. A library name that is not a literal, or a library
  declared other than at module level, is E0430 under FFI-01.
- **Functions.** `NAME.function.argtypes` is assigned a list or tuple of
  `ctypes` types, and `NAME.function.restype` a `ctypes` type or `None`,
  each once, at module level, before any call of the function at module
  level. A call of a function without both, or one that comes before
  them, is E0430 under FFI-02. The checker reads the declarations from
  the module's statements, which then do nothing when the program runs.
- **Calls.** `NAME.function(arguments)` takes positional arguments of the
  types the table gives, checked when the program is compiled: an
  argument of another type is E0404 and a wrong number of arguments E0404,
  where CPython raises `ctypes.ArgumentError`. A `c_bool` argument takes
  any value a truth test takes. Each conversion is the table's, and an
  `int` too large for a `float` raises `ctypes.ArgumentError` with
  CPython's text, `argument 1: OverflowError: int too large to convert to
  float`.
- **Values.** A library and a function are not values otherwise: storing,
  passing or printing one is planned for 1.0 (FFI-09).
- **Text for C.** Every `str` and `bytes` object keeps a zero after its
  data, so a `c_char_p` argument is the object's own buffer, as CPython
  passes it, and stays valid while the object lives.
- **Linking (FFI-04).** Each library a module declares is linked
  dynamically as `-l:library` unless `typy.toml` says otherwise:

  ```toml
  library-paths = ["lib"]

  [libraries."libtools.so.3"]
  link = "static"
  ```

  A static library is linked as `-l:BASE.a`, where `BASE` is the name up
  to `.so`, so `libtools.so.3` links `libtools.a`. `library-paths` names
  directories under the project root where the linker looks first, as
  `-L`. A library or a function the linker cannot find is E0702 when the
  program is built.

## Consequences

- A call of a C function costs what a call from C costs, with each
  conversion a few instructions; the program loads no library at run
  time but those the system loader loads.
- A missing library or function stops the build, where CPython raises
  `OSError` or `AttributeError` when the program runs.
- A wrong argument is found when the program is compiled, so the
  `ctypes.ArgumentError` of a wrong type never arises; the one that can is
  the `float` conversion's.
- A program whose declarations depend on a condition, or come after a
  module-level call, is refused, though CPython runs it.
- The library's own binding, written by a library build (ADR-0046),
  compiles as any module does.

## Related requirements

FFI-01, FFI-02, FFI-03, FFI-04, FFI-08, FFI-09, STD-07, ADR-0003,
ADR-0046.
