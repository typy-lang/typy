# Functions

## What TyPy accepts

| Item | Rule |
| --- | --- |
| Function definitions at the top of the module, with annotated parameters and return | FUN-01, TYP-01 |
| Recursion; running out of stack ends the program with `RecursionError` | FUN-02 |
| Positional, keyword and default arguments, defaults evaluated once | EXP-16 |
| Positional-only parameters before `/` and keyword-only parameters after `*`, bound as CPython binds them; a keyword for a positional-only parameter, or a positional argument past the others, is E0404 | FUN-03 |

Nested functions and functions as values come in 0.8 (FUN-04, FUN-05),
and decorators, generators and built-in functions as values in 0.9 (FUN-06
to FUN-08).

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
