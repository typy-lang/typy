# C interoperability

`typy build --lib` builds a module as a C library that C, C++, Rust and
Python programs call as they call any C library (CLI-09, FFI-05). Calling C
libraries from a TyPy program through the standard `ctypes` module comes
later in 0.5 (FFI-01 to FFI-03).

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `typy build --lib static FILE` writes `libNAME.a`, and `--lib shared` writes `libNAME.so`, beside the file or at `-o PATH`, for the module `NAME` | CLI-09 |
| A library with a version is named as Linux names libraries, such as `libNAME.so.5.0` with links | CLI-13 |
| The functions the module's `__all__` lists are exported under their own names, and the build writes the C header `NAME.h` beside the library | FFI-05 |
| The build also writes `NAME_binding.py`, a module that loads the library with `ctypes` and defines a typed function for each export, and its stub `NAME_binding.pyi` | FFI-08 |
| `int`, `float` and `bool` are `int64_t`, `double` and `bool` in C, and a result of `None` is `void` | FFI-05 |
| An exception never reaches C: a `SystemExit` ends the process with its status, and any other exception is printed and the process aborts | EXC-05 |

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

The binding loads the library by its soname, as a program loads any C
library, so the dynamic loader finds it where the library is installed,
or in a directory `LD_LIBRARY_PATH` names:

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

## Planned

`ctypes` is refused until later in 0.5, when a TyPy program will also
import a library's binding and call it with the same source as CPython
(FFI-01 to FFI-03, FFI-08, ADR-0032):

<!-- typy-expect: rejected E0201 at 1:1 -->
```python
import ctypes
```
