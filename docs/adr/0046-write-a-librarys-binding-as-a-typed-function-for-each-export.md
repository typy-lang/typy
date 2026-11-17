# ADR-0046: Write a library's binding as a typed function for each export

## Status

Accepted

## Date

2026-11-17

## Context

ADR-0032 decides that a library build writes, beside the library and its C
header, a `ctypes` module that loads the library and declares its
functions, and a `.pyi` stub of them, so CPython and TyPy programs call the
library with the same source (FFI-08). It leaves open what the files are
named, how the module finds the library, and how it declares a function.

Three facts shape the answer. The library is built from a module `NAME`
whose file `NAME.py` sits beside it, so a binding of the same name would
take its place; and CPython's import system tries an extension module
before a source module of the same name, so a binding `libNAME.py`
beside `libNAME.so` is never imported: CPython loads the library as an
extension module and fails with `ImportError`. TyPy will compile
`ctypes` only in the forms FFI-01 and FFI-02 name: a `CDLL` call at
module level with a string literal, and `argtypes` and `restype`
assigned at module level before the first call. And a function a `CDLL`
object gives has no annotations, so a type checker sees its result as
`Any`.

## Options considered

For the names:

1. **`NAME_binding.py` and `NAME_binding.pyi`** beside `NAME.py` and the
   library, a name no file of the build has otherwise.
2. **`libNAME.py` and `libNAME.pyi`**, named as the library is, which the
   import system never reaches beside `libNAME.so`.

For each function:

1. **The `CDLL` function itself**, `add = _library.add` after its
   `argtypes` and `restype`, typed only by the stub.
2. **A function of the module's own annotations** that calls the `CDLL`
   function, so the binding holds the types the stub repeats.

For finding the library:

1. **A path beside the binding**, built from `__file__`. TyPy cannot
   compile a `CDLL` call whose argument is not a literal.
2. **The soname**, the literal the dynamic loader looks up, as a program
   names any installed C library.

## Decision

TyPy uses option 1 for the names and option 2 for the functions and for
finding the library.

- **The files.** The build writes `NAME_binding.py` and
  `NAME_binding.pyi` beside the library, for both kinds of library.
- **The module.** It imports `ctypes`, loads the library with
  `ctypes.CDLL` and the soname a shared library of the build has
  (ADR-0045), such as `"libtools.so.5"`, assigns each export's `argtypes`
  and `restype` from its C types (`c_int64`, `c_double`, `c_bool`, or
  `None` for `void`), and defines for each export a function of the same
  name, parameters and annotations, without defaults, that calls it.
  `__all__` lists the exports. The variable that holds the library is
  `_library`, with underscores added until no export or parameter has its
  name.
- **The stub** declares each function with its annotations and `...`.
- **Loading.** CPython finds the library as the dynamic loader finds any
  library: installed in a directory it searches, or in `LD_LIBRARY_PATH`.
  A TyPy program that imports the binding links the library as FFI-01 and
  FFI-04 decide.

## Consequences

- `import tools_binding` gives typed functions under CPython, which a type
  checker reads from the module or its stub.
- A library built with `--lib static` gets the same binding, which loads
  the shared library of the same release; CPython needs that one.
- A call from CPython goes through a Python function and `ctypes`; a call
  from TyPy, once it compiles `ctypes`, is a direct C call.
- An export's default values do not reach the binding, since C has none.

## Related requirements

FFI-01, FFI-02, FFI-04, FFI-08, CLI-13, ADR-0032, ADR-0045.
