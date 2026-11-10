# Exceptions

`raise`, `try` and exception classes come in 0.3 (EXC-01 to EXC-04,
SMT-06, CST-04, CLS-03).

## What TyPy 0.2 accepts

The runtime raises the exceptions CPython raises, with CPython's messages,
and an uncaught exception ends the program as CPython ends it: a traceback
on standard error whose last line matches CPython's, and exit status 1
(STD-02). A program that finishes without an uncaught exception prints
what CPython prints:

<!-- typy-expect: accepted -->
```python
def safe_divide(a: int, b: int) -> float:
    if b == 0:
        return 0.0
    return a / b


print(safe_divide(7, 2), safe_divide(1, 0))
```

## Planned

`try` is refused until 0.3:

<!-- typy-expect: rejected E0201 at 1:1 -->
```python
try:
    print(1)
except ValueError:
    print(2)
```
