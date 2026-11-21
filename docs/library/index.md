# Library reference

TyPy provides Python's built-in types, functions and exceptions, and the
modules of the standard library listed below, each with CPython's names,
results and errors. It adds no module of its own. Where a function behaves
as CPython's does, the
[Python Standard Library](https://docs.python.org/3/library/index.html)
documents it.

| Page | Area | Rules |
| --- | --- | --- |
| [Built-in types](built-in-types.md) | `int`, `float`, `str`, `bytes`, lists, tuples, dicts and sets | DAT |
| [Exceptions](exceptions.md) | Raising, catching and `with`, and the built-in exception classes | EXC |
| [Built-in functions and modules](functions-and-modules.md) | Builtins, methods of the built-in types, `%` formatting, `math`, `sys` and files | LIB |

TyPy 0.10 provides these modules of the standard library:

| Module | What TyPy provides | Described in |
| --- | --- | --- |
| `math` | Its constants and its functions of numbers | [Built-in functions and modules](functions-and-modules.md#the-math-module) |
| `sys` | `argv`, `exit`, the standard streams, the version and the recursion limit | [Built-in functions and modules](functions-and-modules.md#the-sys-module) |
| `dataclasses` | `dataclass` and `field` | [Classes](../reference/classes.md#dataclasses) |
| `typing`, `collections.abc` and `types` | Names for annotations, such as `Iterator`, `Final` and `Protocol` | [Modules](../reference/modules.md) |
| `ctypes` | Calls into C libraries, with C objects, pointers, arrays and structures | [C interoperability](../extending/c-interop.md) |

A module planned for a later release, such as `os`, `time` or `random`, is
refused with E0201.
