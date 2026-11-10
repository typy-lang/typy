# Functions

## What TyPy 0.2 accepts

| Item | Rule |
| --- | --- |
| Function definitions at the top of the module, with annotated parameters and return | FUN-01, TYP-01 |
| Recursion; running out of stack ends the program with `RecursionError` | FUN-02 |
| Positional, keyword and default arguments, defaults evaluated once | EXP-16 |

Keyword-only and positional-only parameters, nested functions, functions
as values, decorators and generators come in 1.0 (FUN-03 to FUN-07).

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
