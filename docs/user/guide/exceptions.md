# Exceptions

Exceptions are standard Python exceptions. You raise them with `raise`,
build them by calling an exception class, and an exception nothing catches
ends the program as CPython ends it.

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `raise` with an exception, or with a class, which is called with no arguments | SMT-06 |
| `raise ... from cause` and `raise ... from None` | EXC-04 |
| The built-in exception classes below, as values, annotations and calls | EXC-02, EXC-10 |
| An exception stored where one of its bases is declared | EXC-01 |
| `str`, `repr`, `print` and f-strings of an exception, from its arguments | EXC-08 |
| `type(e)`, the class the exception was made from | CLS-12 |

An uncaught exception prints a traceback on standard error whose last line
matches CPython's, and the program exits with status 1 (STD-05). A cause
named with `from` is printed first, followed by `The above exception was
the direct cause of the following exception:`, as CPython prints it.

## Built-in exception classes

| Class | Base |
| --- | --- |
| `BaseException` | `object` |
| `Exception` | `BaseException` |
| `ArithmeticError`, `AssertionError`, `AttributeError`, `LookupError` | `Exception` |
| `RuntimeError`, `StopIteration`, `TypeError`, `ValueError` | `Exception` |
| `OverflowError`, `ZeroDivisionError` | `ArithmeticError` |
| `IndexError`, `KeyError` | `LookupError` |
| `NotImplementedError`, `RecursionError` | `RuntimeError` |
| `UnicodeError` | `ValueError` |
| `UnicodeDecodeError`, `UnicodeEncodeError` | `UnicodeError` |

An exception class takes positional arguments of any type. The text of the
exception follows them as in CPython: no argument gives an empty text, one
gives its `str`, and several give the tuple. A `KeyError` shows the `repr`
of its key.

## Example

<!-- typy-expect: accepted -->
```python
def withdraw(balance: int, amount: int) -> int:
    if amount > balance:
        raise ValueError("insufficient funds", amount - balance)
    return balance - amount


error: LookupError = KeyError("owner")
print(error, repr(error), type(error).__name__)
print(withdraw(10, 3))
```

## What TyPy refuses

`raise` takes an exception or an exception class, so raising a message
fails with E0415:

<!-- typy-expect: rejected E0415 at 1:7 -->
```python
raise "insufficient funds"
```

A bare `raise` re-raises the exception being handled, so it is refused
outside an `except` clause with E0416. `UnicodeDecodeError` and
`UnicodeEncodeError` take five arguments that TyPy does not check, so a
program cannot call them (E0417); it can still catch them.

## Planned

`try`, `except`, `else` and `finally` come in 0.3 (CST-04), with exception
classes of your own (CLS-03, CLS-13, CLS-14) and `assert` (SMT-04):

<!-- typy-expect: rejected E0201 at 1:1 -->
```python
try:
    print(1)
except ValueError:
    print(2)
```

The `args` attribute comes in 0.4 (EXC-11). Exception groups and
`except*` are planned for after 1.0.
