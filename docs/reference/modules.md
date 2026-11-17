# Modules

A TyPy program is its main file and every module it imports. The main
file runs as `__main__` (STD-06), and each module runs once, at its first
import, as in CPython (MOD-03).

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `import m`, `import a.b`, `import a.b as c` | SMT-07 |
| `from m import name`, with `as`, for a function, a class, a module variable or a submodule | SMT-07 |
| Modules as `.py` files under the main file's directory or a directory `module-paths` in `typy.toml` names, and packages as directories with an `__init__.py` | MOD-01, MOD-02 |
| Relative imports inside a package, such as `from . import helper` and `from ..tools import text` | MOD-02 |
| A module object: `m.name` reads and assigns a module variable, and the object can be printed, bound to another name and compared with `is` | MOD-10 |

`import tools.text` runs `tools/__init__.py`, then `tools/text.py`, and
binds `tools`. A package is preferred to a module of the same name, and a
module of the program hides a standard library module of the same name,
as CPython's search order does.

`from m import x` copies the value `x` has when the import runs, while
`m.x` reads the variable as it is when it runs, so a later assignment in
`m` shows through `m.x` only. Reading a module variable that is not bound
yet raises `AttributeError` through `m.x`, and `ImportError` through
`from m import x`, with CPython's messages.

A module whose code raises is not kept: the next import runs its code
again. An import inside a function runs when the function is called, and
binds a variable of the function.

## Module objects

A module object prints as CPython's does, with the path of the module's
file from the main file's directory, where CPython shows the absolute
path (OI-20): `<module 'tools.text' from 'tools/text.py'>`. A module's
`__name__` is its qualified name, and the main file's is `__main__`:

<!-- typy-expect: accepted -->
```python
def main() -> None:
    print("running as", __name__)


if __name__ == "__main__":
    main()
```

## Lazy imports

A lazy import binds its names without running the module: the module runs
at the first use of one of them, as PEP 810 defines for Python 3.15
(MOD-06). `lazy import m` and `lazy from m import name` need the Python
floor 3.15, set with `python-floor = "3.15"` in `typy.toml`. At any
floor, a module-level `__lazy_modules__` list makes the imports of the
modules it names lazy (MOD-07):

<!-- typy-expect: accepted -->
```python
__lazy_modules__ = ["json_tools"]
```

In this program of two files, `main.py` prints `before use`, then
`heavy runs`, then `42`:

```text
# heavy.py
print("heavy runs")
VALUE = 42

# main.py
__lazy_modules__ = ["heavy"]
import heavy

print("before use")
print(heavy.VALUE)
```

An exception the module raises while it runs reaches the first use of the
name, and the next use runs the module again (MOD-08). Assigning the name
before its first use cancels its import, and the module does not run for
it. CPython 3.12 to 3.14 ignore `__lazy_modules__` and run such a module
at its import, so a program whose output depends on when a module runs
prints differently there.

A lazy import is allowed at module level, also inside `if`, `for`,
`while` and `with`. As in CPython, it is E0103 inside a function, a
`try` statement, or with `*` or `__future__`; inside a class it is E0202,
since a class body holds only its methods.

Another module may use a name a module imports lazily, as
`from app import heavy` or `app.heavy` does: reading it runs the import
first, as in CPython. Assigning `m.x = v` through a lazily imported `m`
runs `m`'s import before the assignment. A variable may be bound by more
than one lazy import, and its first use runs the one that ran last
(MOD-11). So may a function or a class, as the next section describes.

## A name bound by more than one import

A function or a class may be bound by several imports, eager or lazy, such
as one in each branch of an `if`, and a call runs the one the import that
ran last bound, as in CPython (MOD-12). A name an import inside a block
binds raises `NameError` when that import has not run:

<!-- typy-expect: accepted -->
```python
import sys

if len(sys.argv) > 9:
    from math import sqrt as root
else:
    from math import isqrt as root
print(root(17))
if len(sys.argv) > 9:
    from math import floor
try:
    print(floor(2.5))
except NameError as error:
    print(error)
```

The arguments of such a call are checked against each function it may
run. A name bound to more than one function or class read as a value,
named in an annotation or used from another module is planned for 1.0
(E0201, MOD-14).

The switches of `sys` that change lazy imports from outside the module,
`sys.set_lazy_imports`, `sys.set_lazy_imports_filter`, their `get_`
companions and `sys.lazy_modules`, are E0202 (MOD-09): whether an import
is lazy is decided where it is written.

## What is refused

Two modules that import each other at module level are a circular import,
E0422, shown with each import of the cycle (MOD-05). Moving one of the
imports into the function that uses the module breaks the cycle, as in
CPython (MOD-13); inside `__init__` it is planned for 1.0 (E0426,
MOD-15). A name a module does not define is E0425, at compile time.

A module keeps one definition per name, as for `def` (STD-04). A name
that an import binds to a function or a class and that the module also
defines or assigns is E0202. Import it under another name:

<!-- typy-expect: rejected E0202 at 1:18 -->
```python
from math import sqrt


def sqrt(value: float) -> float:
    return value / 2
```

`from m import *` is planned for after 1.0. Standard library modules come
with their releases: `math` and `sys` in 0.3, `dataclasses` in 0.4, and
`ctypes` in 0.5 (LIB-04 to LIB-07, FFI-01); `os`, `time` and `random`
are planned for 1.0. `typing` and
`collections.abc` give `Iterator` and `Iterable`, `typing` gives
`ClassVar`, `Final` and `Protocol`, and `types` gives `TracebackType`
and `ModuleType`, for annotations (TYP-15, LIB-06). `dataclasses` gives
`dataclass` and `field`, as [Classes](classes.md#dataclasses) describes
(CLS-05). The library reference describes
[`math`](../library/functions-and-modules.md#the-math-module) and
[`sys`](../library/functions-and-modules.md#the-sys-module), and
[C interoperability](../extending/c-interop.md) describes `ctypes`. A
module planned for a later release is refused:

<!-- typy-expect: rejected E0201 at 1:1 -->
```python
import time
```
