# C interoperability

A TyPy program calls C libraries through the standard `ctypes` module, so
the same program runs under CPython, and `typy build --lib` builds a module
as a C library that C, C++, Rust and Python programs call as they call any
C library (FFI-01 to FFI-05, CLI-09).

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `typy build --lib static FILE` writes `libNAME.a`, and `--lib shared` writes `libNAME.so` on Linux and `libNAME.dylib` on macOS, beside the file or at `-o PATH`, for the module `NAME` | CLI-09 |
| A library with a version is named as Linux names libraries, such as `libNAME.so.5.0` with links | CLI-13 |
| A library with a version for macOS is named as macOS names libraries, such as `libNAME.5.dylib` with the link `libNAME.dylib` | CLI-16 |
| The functions the module's `__all__` lists are exported under their own names, and the build writes the C header `NAME.h` beside the library | FFI-05 |
| The build also writes `NAME_binding.py`, a module that loads the library with `ctypes` and defines a typed function for each export, and its stub `NAME_binding.pyi` | FFI-08 |
| `int`, `float` and `bool` are `int64_t`, `double` and `bool` in C, and a result of `None` is `void` | FFI-05 |
| An exception never reaches C: a `SystemExit` ends the process with its status, and any other exception is printed and the process aborts | EXC-05 |
| `NAME = ctypes.CDLL("library")` or `ctypes.cdll.LoadLibrary("library")` at module level declares a library, which the build links | FFI-01 |
| `ctypes.CDLL(None)` declares the C library and the C maths library, which every program links, on every system | FFI-14 |
| A C function whose `argtypes` and `restype` are assigned at module level is called directly, with the conversions `ctypes` defines | FFI-02, FFI-03 |
| `ctypes` objects, pointers and arrays hold C memory as in CPython, and a C function writes them through `byref` and `POINTER` parameters | FFI-03 |
| `ctypes.Structure` subclasses with a literal `_fields_` list are laid out as C lays out a struct, and pass to C by pointer or by value | FFI-03 |
| `typy.toml` says whether each library is linked statically or dynamically, and where the linker looks | FFI-04 |

## Building a library

A library is a module whose `__all__` lists the functions it exports.
Save this as `geometry.py`:

<!-- typy-expect: accepted -->
```python
__all__ = ["area", "is_square"]

calls = 0


def area(width: float, height: float) -> float:
    global calls
    calls += 1
    return width * height


def is_square(width: int, height: int) -> bool:
    return width == height
```

`typy build --lib static geometry.py` writes `libgeometry.a` and
`geometry.h`, which declares:

```text
double area(double width, double height);
bool is_square(int64_t width, int64_t height);
```

A C program includes the header and links the library and the C maths
library, which the runtime in the library calls:

```sh
cc -o shapes shapes.c -L. -lgeometry -lm -Wl,--gc-sections
```

`--gc-sections` leaves out the parts of the runtime that no export
reaches.

## Calling a library from Python

`typy build --lib shared geometry.py` writes `libgeometry.so`, and beside
it `geometry_binding.py`, a module that loads the library with `ctypes`
and defines a function of the same name and types for each export, and
its stub `geometry_binding.pyi`, which type checkers read. A Python
program imports the binding:

```text
>>> import geometry_binding as geometry
>>> geometry.area(2.0, 3.5)
7.0
>>> geometry.is_square(3, 3)
True
```

The binding loads the library by its soname, or on macOS its install
name, as a program loads any C library, so the dynamic loader finds it
where the library is installed, or in a directory `LD_LIBRARY_PATH`
names, `DYLD_LIBRARY_PATH` on macOS:

```sh
LD_LIBRARY_PATH=$PWD python3 shapes.py
```

A library built with `--lib static` gets the same binding, which loads
the shared library of the same release, since CPython loads only shared
libraries.

## Naming a library by its version

A library's files carry the release of its version: the module's
`__version__`, or else `version` in `typy.toml`, for a module that has
none. The release is the numbers the version starts with, so
`__version__ = "5.0rc1"` names the files after `5.0`:

| Kind | Library | Links | Soname |
| --- | --- | --- | --- |
| `--lib shared` | `libgeometry.so.5.0` | `libgeometry.so.5` and `libgeometry.so` | `libgeometry.so.5` |
| `--lib static` | `libgeometry.5.0.a` | `libgeometry.a` | none |

`-lgeometry` finds the library through its link, and a program linked
with a shared library loads it by its soname, so it keeps working when
release 5.1 replaces 5.0. Without a version, the files are
`libgeometry.so` and `libgeometry.a`. With `-o PATH`, the library is
written at that path alone. A `__version__` that does not start with a
number, or that differs from `version` in `typy.toml`, is E0429.

On macOS a shared library is named by its major version alone, which is
also its install name, and records the release as its versions; a static
library is `libgeometry.a` whatever its version (ADR-0056):

| Kind | Library | Link | Install name | Versions |
| --- | --- | --- | --- | --- |
| `--lib shared` | `libgeometry.5.dylib` | `libgeometry.dylib` | `libgeometry.5.dylib` | compatibility 5.0, current 5.0 |
| `--lib static` | `libgeometry.a` | none | none | none |

Release 5.1 rewrites `libgeometry.5.dylib` with the current version 5.1,
and a program linked with release 5.0 loads it.

## How a library runs

The module runs once, when the library is loaded: when the program starts,
for a library it links, or when `ctypes.CDLL` loads it. That is when CPython
runs a module's body, on import, so the module's variables hold their
values before the first call. The module's `__name__` is its own name, so
code under `if __name__ == "__main__":` does not run, and `sys.argv` is
`['']`. What the module and the exports print is written before each call
returns, so it keeps its order with what the program prints, as long as the
program writes its own output before it calls the library, as
`fflush(stdout)` or `python3 -u` does.

An export's name is its C symbol. Its parameters and result are `int`,
`float` and `bool`, and it may return `None`. A function that takes or
returns other values needs an export of these types that calls it. An
`int` result outside `int64_t` raises `OverflowError`, which ends the
process as any exception does. The runtime is not safe to call from two
threads at once, so a program calls a library's exports from one thread at
a time; each call may come from a different thread. Each library holds its
own copy of the runtime, so a program may link several TyPy libraries.

An export may not take a name the runtime calls in the C library, such as
`read`, `free` or `sqrt`, or one C keeps for itself, such as `__init`; the
build reports E0429 and names the function. Building with `--lib` checks
the names: `typy check --lib static geometry.py` reports the same
diagnostics without writing a library.

## Calling C through ctypes

A program declares a library and each function it calls, as a program for
CPython does, and TyPy compiles each call into a direct call of the C
function, with the library linked when the program is built (ADR-0047):

<!-- typy-expect: accepted -->
```python
import ctypes

libm = ctypes.CDLL(None)
libm.cbrt.argtypes = [ctypes.c_double]
libm.cbrt.restype = ctypes.c_double


def side(volume: float) -> float:
    return libm.cbrt(volume)


print(side(27.0), side(1000.0))
```

A library is named by a string literal, the file name its system gives
it, such as `libm.so.6` on Linux or `libm.dylib` on macOS, or by `None`.
`ctypes.CDLL(None)` declares the C library and the C maths library, which
every program links, so a program that calls only their functions builds
and runs alike on Linux and macOS, as under CPython, where it calls the
functions the process has loaded (FFI-14). Both `argtypes` and `restype`
are assigned at module level, once, before the function is called at
module level; a declaration that breaks one of these is E0430.
An argument of a type the declaration does not take is E0404 when the
program is compiled. Each value converts as `ctypes` converts it:

| Type | Argument | Result |
| --- | --- | --- |
| `c_int8` to `c_uint64`, `c_byte`, `c_short`, `c_int`, `c_long`, `c_longlong` and their unsigned forms, `c_size_t`, `c_ssize_t` | An `int` or a `bool`, wrapped to the type's width | An `int` |
| `c_float`, `c_double` | An `int`, a `float` or a `bool` | A `float` |
| `c_bool` | Any value, by its truth | A `bool` |
| `c_char_p` | `bytes`, whose own buffer C reads, or `None` | `bytes`, or `None` for null |
| `c_void_p` | An `int`, `bytes` or `None` | An `int`, or `None` for null |

An `int` too large for a `float` raises `ctypes.ArgumentError`, as in
CPython. A library or a function the linker cannot find stops the build
with E0702, where CPython raises `OSError` or `AttributeError` when the
program runs.

## Objects, pointers and arrays

A `ctypes` object holds a C value in memory a C function can write. A
parameter declared as `ctypes.POINTER(T)` takes the object's address,
whether the program passes `ctypes.byref(x)`, the object itself, an array
of `T`, a pointer from `ctypes.pointer(x)`, or `None` for null (ADR-0048):

<!-- typy-expect: accepted -->
```python
import ctypes

libm = ctypes.CDLL(None)
libm.frexp.argtypes = [ctypes.c_double, ctypes.POINTER(ctypes.c_int)]
libm.frexp.restype = ctypes.c_double

exponent = ctypes.c_int()
mantissa = libm.frexp(48.0, ctypes.byref(exponent))
print(mantissa, exponent.value, exponent)

Numbers = ctypes.c_int * 4
numbers = Numbers(1, 2, 3)
numbers[3] = numbers[0] + len(numbers)
print(list(numbers), sum(numbers), ctypes.sizeof(numbers))

pointer = ctypes.pointer(exponent)
pointer[0] += 1
print(exponent.value, pointer.contents)
```

| Operation | Gives |
| --- | --- |
| `T(value)`, `T()` | An object of a scalar type holding the value converted as an argument is, or zero, `None` or `False` |
| `x.value`, `x.value = v` | The value as a result of type `T` is, and a new value converted and wrapped to the type's width |
| `ctypes.pointer(x)`, `POINTER(T)(x)`, `POINTER(T)()` | A pointer that keeps `x` alive, or a null pointer |
| `p.contents`, `p[i]`, `p[i] = v` | The object pointed to, and the value of an item; a null pointer raises `ValueError` |
| `T * n`, `(T * n)(items)` | An array class of `n` items, `n` an integer literal, and an array, the rest zero |
| `a[i]`, `a[i] = v`, `len(a)`, `for v in a`, `v in a` | Items counted from the end when negative; one outside raises `IndexError` |
| `ctypes.sizeof(T)`, `ctypes.sizeof(x)` | The bytes of the class on 64-bit Linux |
| `repr(x)`, `bool(x)`, `x == y` | As in CPython: `c_long(7)` for a `c_int64`, the value's truth, and identity |

A `POINTER(T)` result is a new pointer over memory the C library owns.
A class is bound to a name, called, multiplied into an array class or
named in a declaration; using it any other way is E0201 under FFI-10.

## Structures

A class whose one base is `ctypes.Structure`, with a `_fields_` list
written in the class, is a C struct: each field sits where a C compiler
puts it, and a C function takes the structure by pointer or by value
(ADR-0049):

<!-- typy-expect: accepted -->
```python
import ctypes

libc = ctypes.CDLL(None)


class Division(ctypes.Structure):
    _fields_ = [("quotient", ctypes.c_int), ("remainder", ctypes.c_int)]


class Segment(ctypes.Structure):
    _fields_ = [("start", Division), ("ends", ctypes.c_int * 2)]


libc.div.argtypes = [ctypes.c_int, ctypes.c_int]
libc.div.restype = Division

result = libc.div(17, 5)
print(result.quotient, result.remainder)
segment = Segment(result, ends=(ctypes.c_int * 2)(1, 2))
segment.start.quotient += 10
print(segment.start.quotient, list(segment.ends), ctypes.sizeof(Segment))
```

`S(...)` takes the fields in order, then by name; too many values, a name
that is not a field, or a field given twice is E0404. A scalar field reads
and assigns as `value` does. A structure, array or pointer field reads as
an object over the field's memory, so a change through it changes the
structure, and assigning one copies it. A structure passed to or returned
from a C function by value follows the target's C calling convention:
System V on x86_64 Linux, AAPCS64 on AArch64 Linux, and Apple's arm64
convention on macOS (FFI-12, FFI-13).

## Linking a library

A library is linked dynamically unless `typy.toml` says otherwise, and the
system loads it when the program starts. A project that ships its own
libraries names where the linker looks, and which to link into the
program:

```toml
library-paths = ["lib"]

[libraries."libtools.so.2"]
link = "static"
```

A static library is the archive named like the shared one up to `.so`, so
`libtools.so.2` links `lib/libtools.a`; on macOS, `libtools.2.dylib` links
the `libtools.a` of the first library path that holds it. A TyPy library's binding,
`tools_binding.py`, loads the library by its soname, so a TyPy program that
imports the binding calls the library directly, linked as `typy.toml`
says, and CPython runs the same program through `ctypes`.

## Planned

Planned for 1.0 are methods and class attributes other than `_fields_`
in a structure, `_fields_` assigned after the class statement, bit
fields, a subclass of a structure, and `ctypes.Union` (FFI-11); and
`c_char`, `c_wchar`, `c_wchar_p`, `create_string_buffer`, `cast` and
`addressof`, pointers to and arrays of pointers and arrays, arrays whose
length is not an integer literal, array slices, `byref` other than as a C
function's argument, assigning a pointer's `contents`, and `ctypes`
classes as values, in annotations, `isinstance` and `type` (FFI-10), so a
function takes and returns the values of an object rather than the object.
A library or a C function used as a value, stored, passed or printed, is
planned for 1.0 too (FFI-09):

<!-- typy-expect: rejected E0201 at 4:7 -->
```python
import ctypes

libc = ctypes.CDLL("libc.so.6")
print(libc)
```
