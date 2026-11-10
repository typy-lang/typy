# Expressions

Operators give Python's results, and operands are evaluated in Python's
order.

## What TyPy 0.2 accepts

| Item | Rule |
| --- | --- |
| Evaluation in the order the Language Reference defines | EXP-01 |
| `+`, `-`, `*`, `/`, `//`, `%` and `**` with Python's results for `int` and `float` | EXP-02 |
| `ZeroDivisionError` for division and modulo by zero | EXP-03 |
| `x ** y` when CPython's result has one type TyPy represents | EXP-04 |
| Shifts and bitwise operators on `int` | EXP-05 |
| Comparisons between values of one type and between `int` and `float`, chained | EXP-06 |
| `in` and `not in` on `str`, `bytes`, `list`, `tuple`, `dict` and `set` | EXP-07 |
| `is` and `is not` when one side is `None` or both are instances or type objects | EXP-08 |
| `and` and `or` with operands of one type | EXP-09 |
| Truth tests of numbers, strings, containers, optional values and instances | EXP-10 |
| `x if c else y` with branches of one type | EXP-11 |
| List, set and dict comprehensions, each with its own scope | EXP-12 |
| Positional, keyword and default arguments | EXP-16 |

Assignment expressions `:=` come in 0.5 (EXP-13), and `lambda`, generator
expressions and `*args` in 1.0 (EXP-14, EXP-15, EXP-17).

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
