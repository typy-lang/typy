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

TyPy 0.12 provides these modules of the standard library:

| Module | What TyPy provides | Described in |
| --- | --- | --- |
| `math` | Its constants and its functions of numbers | [Built-in functions and modules](functions-and-modules.md#the-math-module) |
| `sys` | `argv`, `exit`, the standard streams, the version and the recursion limit | [Built-in functions and modules](functions-and-modules.md#the-sys-module) |
| `dataclasses` | `dataclass` and `field` | [Classes](../reference/classes.md#dataclasses) |
| `abc` | `ABC` and `abstractmethod` | [Classes](../reference/classes.md#abstract-classes) |
| `enum` | `Enum`, `IntEnum`, `StrEnum`, `Flag`, `IntFlag` and `auto` | [Classes](../reference/classes.md#enum-classes) |
| `bisect` | `bisect`, `bisect_left`, `bisect_right`, `insort`, `insort_left` and `insort_right` | [Built-in functions and modules](functions-and-modules.md#the-bisect-module) |
| `heapq` | The heap functions, `merge`, `nlargest` and `nsmallest` | [Built-in functions and modules](functions-and-modules.md#the-heapq-module) |
| `string` | Its constants, `capwords` and `Template` | [Built-in functions and modules](functions-and-modules.md#the-string-module) |
| `base64` | The Base64, Base32, Base16, Ascii85 and Base85 encoders and decoders | [Built-in functions and modules](functions-and-modules.md#the-base64-module) |
| `textwrap` | `TextWrapper`, `wrap`, `fill`, `shorten`, `dedent` and `indent` | [Built-in functions and modules](functions-and-modules.md#the-textwrap-module) |
| `binascii` | `a2b_base64`, `b2a_base64`, `hexlify`, `unhexlify`, `crc32`, their aliases and `Error` | [Built-in functions and modules](functions-and-modules.md#the-binascii-module) |
| `typing`, `collections.abc` and `types` | Names for annotations, such as `Iterator`, `Final` and `Protocol` | [Modules](../reference/modules.md) |
| `ctypes` | Calls into C libraries, with C objects, pointers, arrays and structures | [C interoperability](../extending/c-interop.md) |

A module or name planned for a later release is refused with E0201,
naming the release: `statistics`, `time` and `random` (LIB-30),
`copy` (LIB-33), `itertools`, `collections`, `datetime`, `pathlib`,
`os.path`, `fractions` and `re` (LIB-35), and `warnings` (LIB-38) in
0.13; `json`, `functools`, `argparse`, `logging` and `csv` (LIB-36) in
0.14; the rest of `os` (LIB-07) in 1.0; and `doctest` in a release after
1.0. Each other module is E0206. From the Python floor
3.15, `math` also gives `math.integer` (LIB-26).
