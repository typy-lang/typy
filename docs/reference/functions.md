# Functions

## What TyPy accepts

| Item | Rule |
| --- | --- |
| Function definitions at the top of the module, with annotated parameters and return | FUN-01, TYP-01 |
| Recursion; running out of stack ends the program with `RecursionError` | FUN-02 |
| Positional, keyword and default arguments, defaults evaluated once | EXP-16 |
| Positional-only parameters before `/` and keyword-only parameters after `*`, bound as CPython binds them; a keyword for a positional-only parameter, or a positional argument past the others, is E0404 | FUN-03 |
| A function of the program as a value of type `Callable[[A, ...], R]`: stored, passed, returned, called, compared and hashed | FUN-05, NFR-19 |

Nested functions, methods as values and the `key` argument of `sorted`,
`list.sort`, `min` and `max` are planned for 0.8 (FUN-04, FUN-05), and
decorators, generators and built-in functions as values for 0.9 (FUN-06
to FUN-08).

## Functions as values

A function's name read without a call is its function object, a value of
type `Callable[[A, ...], R]` of `typing` or `collections.abc`, where `A,
...` are its parameter types and `R` its result type. A variable, a list
item, a dict key, an attribute or an argument holds it, and a call through
it passes one argument by position for each parameter:

<!-- typy-expect: accepted -->
```python
from typing import Callable


def double(value: int) -> int:
    return value * 2


def square(value: int) -> int:
    return value * value


def apply(function: Callable[[int], int], value: int) -> int:
    return function(value)


operations = [double, square]
names = {double: "double", square: "square"}
print([apply(each, 3) for each in operations], names[square])
print(double is operations[0], double == square)
```

As in CPython, each function has one object, so `is`, `==` and `hash`
compare functions by identity, a function value is true, and `repr` writes
`<function double at 0x...>` with the object's address. A function value
has no order, so `<` between two is E0407.

A function fits a `Callable` only when a call through the value converts
nothing (NFR-19). Each argument the `Callable` passes must be a value of
the function's parameter type as it is held, and the function's result a
value of the `Callable`'s result type as it is held. The types may differ
only by a class of the program, since every object is held as its
address: a function of any `Shape` fits a `Callable[[Circle], None]`, and
a function that returns a `Circle` fits a `Callable[[], Shape]`. A
function of an `int | None` does not fit a `Callable[[int], int]`, since
a call by name holds the `int` as an optional value first, and a function
of a `float` does not fit a `Callable[[int], float]`, since an `int` is
never a `float` (TYP-09):

<!-- typy-expect: rejected E0402 at 8:37 -->
```python
from typing import Callable


def scale(value: float) -> float:
    return value * 2.0


converter: Callable[[int], float] = scale
```

A call through a value checks the stack, as a recursive function does, so
recursion through function values ends with `RecursionError` too (FUN-02).

These forms are refused:

| Form | Diagnostic |
| --- | --- |
| `Callable[..., R]`, which takes any arguments, and `Callable` without its types | E0408 |
| A keyword argument through a function value, such as `f(value=1)`, since a `Callable` names no parameters | E0404 |
| A generic function as a value, which has one copy for each set of types | E0202 |
| A function with a keyword-only parameter as a value, since a `Callable` passes every argument by position | E0202 |
| A call of a value that is not a function, such as an `int` | E0401 |

## Example

<!-- typy-expect: accepted -->
```python
def fib(n: int) -> int:
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


def greet(name: str, punctuation: str = "!") -> str:
    return "hello " + name + punctuation


print(fib(20), greet("world"), greet("you", punctuation="?"))
```

A function needs a return annotation, `-> None` when it returns nothing:

<!-- typy-expect: rejected E0204 at 1:5 -->
```python
def show(value: int):
    print(value)
```
