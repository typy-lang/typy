# Expressions

Operators give Python's results, and operands are evaluated in Python's
order.

## What TyPy accepts

| Item | Rule |
| --- | --- |
| Evaluation in the order the Language Reference defines | EXP-01 |
| `+`, `-`, `*`, `/`, `//`, `%` and `**` with Python's results for `int` and `float` | EXP-02 |
| `ZeroDivisionError` for division and modulo by zero | EXP-03 |
| `x ** y` when CPython's result has one type TyPy represents | EXP-04 |
| Shifts and bitwise operators on `int` | EXP-05 |
| Comparisons between values of one type and between `int` and `float`, chained | EXP-06 |
| `in` and `not in` on `str`, `bytes`, `list`, `tuple`, `dict` and `set` | EXP-07 |
| `is` and `is not` when one side is `None`, both are `bool` or `bool \| None`, or both are instances or type objects | EXP-08 |
| `and` and `or` with operands of one type | EXP-09 |
| Truth tests of numbers, strings, containers, optional values and instances | EXP-10 |
| `x if c else y` with branches of one type | EXP-11 |
| List, set and dict comprehensions, each with its own scope | EXP-12 |
| Assignment expressions `name := value`, as CPython scopes them | EXP-13 |
| `lambda` where a `Callable` is expected | EXP-14 |
| Positional, keyword and default arguments | EXP-16 |

An assignment expression `name := value` assigns the variable and gives
the value, as an assignment does (EXP-13). Inside a comprehension it
assigns the variable of the function or module around it. A test of
`(name := value) is not None` narrows `name`, as a test of the variable
does. CPython's syntax errors are E0103: rebinding a name the
comprehension binds, `:=` in a comprehension's source, and `__debug__`
as the name.

<!-- typy-expect: accepted -->
```python
def lookup(table: dict[str, int], key: str) -> int:
    if (found := table.get(key)) is not None:
        return found + 1
    return -1


print(lookup({"a": 1}, "a"), lookup({"a": 1}, "b"))
```

A `lambda` is a function of one expression, made each time the
expression runs, as in CPython. Its parameters have no annotations, so
they take their types, by position, from the `Callable` its context
expects: an annotated variable, a parameter, a function's result, an
item of a typed list or dict, either branch of a conditional, or the
`key` of `sorted`, `list.sort`, `min` and `max`, which passes each item.
Its result takes the expected type, or the type of its body where no
context gives one. A `lambda` reads the variables around it as a nested
function does, so the ones made in a loop or a comprehension read the
last value of its variable:

<!-- typy-expect: accepted -->
```python
from typing import Callable


def adder(step: int) -> Callable[[int], int]:
    return lambda value: value + step


double: Callable[[int], int] = lambda x: x * 2
made = [lambda: i for i in range(3)]
words = ["pear", "fig", "banana"]
print(adder(10)(5), double(21), [each() for each in made])
print(sorted(words, key=lambda word: (len(word), word)))
```

A `lambda` with a parameter where no `Callable` is expected, or with more
parameters than the expected `Callable` passes, is E0203; define the
function with `def`. A parameter taken by keyword only is E0202, since a
`Callable` passes every argument by position. A `lambda` that reads a
variable assigned no type yet where it stands is E0306, since its body is
checked there. A `lambda` inside a generic function is planned for 0.9
(FUN-10). A `lambda` stored in a variable that it calls, such as a
recursive one inside a function, holds itself through the variable's
cell: a reference cycle that cycle collection frees in 1.0 (MEM-06).

Generator expressions and `*args` come in 0.9 (EXP-15, EXP-17).

## Example

<!-- typy-expect: accepted -->
```python
def scale(value: float, factor: float = 2.0) -> float:
    return value * factor


numbers = [5, -3, 8, 0]
evens = [n for n in numbers if n % 2 == 0]
squares = {n: n * n for n in numbers if n > 0}
print(7 // -2, 7 % -2, 2 ** 10, 1 < 2 <= 3, 10 / 4, -8 >> 1)
print(evens, squares, 8 in numbers, scale(1.5), scale(factor=3.0, value=2.0))
print("big" if max(numbers) > 5 else "small", "" or "default")
```

`**` is refused when CPython's result could have another type, such as a
`float` for a negative exponent:

<!-- typy-expect: rejected E0302 at 3:15 -->
```python
base = 2
exponent = -1
print(base ** exponent)
```
