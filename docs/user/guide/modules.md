# Modules

A TyPy program is its main file and every module it imports. The main
file runs as `__main__` (STD-06), and each module runs once, at its first
import, as in CPython (MOD-03).

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `import m`, `import a.b`, `import a.b as c` | SMT-07 |
| `from m import name`, with `as`, for a function, a class, a module variable or a submodule | SMT-07 |
| Modules as `.py` files under the main file's directory, and packages as directories with an `__init__.py` | MOD-01, MOD-02 |
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
floor 3.15. At any floor, a module-level `__lazy_modules__` list makes
the imports of the modules it names lazy (MOD-07):

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
since a class body holds only its methods. TyPy 0.4 adds what 0.3 refuses
with E0201 (MOD-11): using a name another module imports lazily, such as
`from app import heavy`, assigning `m.x = v` through a lazily imported
`m`, and binding one name by two lazy imports.

## What is refused

Two modules that import each other at module level are a circular import,
E0422, shown with each import of the cycle (MOD-05). Moving one of the
imports into the function that uses the module breaks the cycle. A name a
module does not define is E0425, at compile time.

`from m import *` is planned for after 1.0. Standard library modules come
with their releases: `sys` and `math` in 0.3, `dataclasses` and the rest
of `typing` in 0.4 (LIB-04 to LIB-07). In 0.3, `typing` and
`collections.abc` give `Iterator` and `Iterable`, and `types` gives
`TracebackType` and `ModuleType`, for annotations (TYP-15):

<!-- typy-expect: rejected E0201 at 1:1 -->
```python
import math
```
