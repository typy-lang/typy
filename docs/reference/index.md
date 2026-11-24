# Language reference

Each page covers one area of the TyPy specification: what TyPy 0.12 accepts,
with the rule ID of each item, what it refuses, and what later releases
add. The rule IDs are the ones diagnostics name, such as `TYP-03`.

| Page | Area | Rules |
| --- | --- | --- |
| [Standard Python](standard-python.md) | Which programs TyPy accepts, and the promise it makes about them | STD |
| [Lexical structure](lexical-structure.md) | Literals, comments and indentation | LEX |
| [Types and annotations](types.md) | Annotations, inference and narrowing | TYP |
| [Expressions](expressions.md) | Operators, comparisons, comprehensions and calls | EXP |
| [Simple statements](simple-statements.md) | Assignment, unpacking, `pass`, `return`, `break` and `continue` | SMT |
| [Compound statements](compound-statements.md) | `if`, `while`, `for` and `match` | CST |
| [Functions](functions.md) | Function definitions and recursion | FUN |
| [Classes](classes.md) | Classes, attributes, methods and type objects | CLS |
| [Generics](generics.md) | Type parameters and protocols | GEN |
| [Modules](modules.md) | Imports, packages and lazy imports | MOD |
| [Memory](memory.md) | Reference counting, value layout and the allocator | MEM |

The other areas of the specification have their pages in the other
sections of this documentation:

| Page | Area | Rules |
| --- | --- | --- |
| [Built-in types](../library/built-in-types.md) | `int`, `float`, `str`, `bytes`, lists, tuples, dicts and sets | DAT |
| [Exceptions](../library/exceptions.md) | Raising, catching and `with` | EXC |
| [Built-in functions and modules](../library/functions-and-modules.md) | Builtins, methods of the built-in types, `math`, `sys` and files | LIB |
| [Command line](../using/command-line.md) | `typy check`, `build` and `run`, and `typy.toml` | CLI |
| [C interoperability](../extending/c-interop.md) | `ctypes`, and modules built as C libraries | FFI |
| [Diagnostics](../diagnostics/index.md) | How TyPy reports a problem | DIA |
