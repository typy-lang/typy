# Language guide

Each page covers one area of the TyPy specification: what TyPy 0.2 accepts,
with the rule ID of each item, what it refuses, and what later releases
add. The rule IDs are the ones diagnostics name, such as `TYP-03`.

| Page | Area | Rules |
| --- | --- | --- |
| [Standard Python](standard-python.md) | Which programs TyPy accepts, and the promise it makes about them | STD |
| [Command line](command-line.md) | `typy check`, `build` and `run` | CLI |
| [Lexical structure](lexical-structure.md) | Literals, comments and indentation | LEX |
| [Types and annotations](types.md) | Annotations, inference and narrowing | TYP |
| [Built-in types](built-in-types.md) | `int`, `float`, `str`, `bytes`, lists, tuples, dicts and sets | DAT |
| [Expressions](expressions.md) | Operators, comparisons, comprehensions and calls | EXP |
| [Simple statements](simple-statements.md) | Assignment, unpacking, `pass`, `return`, `break` and `continue` | SMT |
| [Compound statements](compound-statements.md) | `if`, `while` and `for` | CST |
| [Functions](functions.md) | Function definitions and recursion | FUN |
| [Classes](classes.md) | Classes, attributes, methods and type objects | CLS |
| [Generics](generics.md) | Type parameters and protocols, planned for 0.4 | GEN |
| [Exceptions](exceptions.md) | Raising and catching, planned for 0.3 | EXC |
| [Modules](modules.md) | Imports, planned for 0.3 | MOD |
| [Built-in functions and library](library.md) | Builtins and methods of the built-in types | LIB |
| [Memory](memory.md) | Reference counting and value layout | MEM |
| [C interoperability](c-interop.md) | `ctypes`, planned for 0.5 | FFI |
| [Diagnostics](diagnostics.md) | How TyPy reports a problem | DIA |
