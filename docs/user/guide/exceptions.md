# Exceptions

Exceptions are standard Python exceptions. You raise them with `raise`,
catch them with `try` and `except`, build them by calling an exception
class, and an exception nothing catches ends the program as CPython ends
it.

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `raise` with an exception, or with a class, which is called with no arguments | SMT-06 |
| `raise ... from cause` and `raise ... from None` | EXC-04 |
| `try` with `except` clauses, `else` and `finally` | CST-04 |
| `except C`, `except (A, B)`, `except C as name` and a bare `except` | EXC-01, EXC-09 |
| A bare `raise` inside an `except` clause, which re-raises the exception | EXC-07 |
| The built-in exception classes below, as values, annotations and calls | EXC-02, EXC-10 |
| An exception stored where one of its bases is declared | EXC-01 |
| `str`, `repr`, `print` and f-strings of an exception, from its arguments | EXC-08 |
| `type(e)`, the class the exception was made from | CLS-12 |

An `except` clause catches an exception of the class it names or of any
class derived from it, so `except LookupError` catches a `KeyError`. The
clauses are tried in order. The name after `as` holds the exception while
the clause runs and is deleted when it ends, as in CPython; with several
classes it has the type of their nearest common base. An exception no
clause catches goes on to the code around the `try`. `else` runs when the
body raised nothing. `finally` runs on every way out of the `try`: after
the body, a clause or `else`, on `return`, `break` and `continue`, and
before an exception goes on. While an exception waits for `finally`, an
exception raised in the block keeps it as its context.

An exception raised while another is handled keeps it as its context. An
uncaught exception prints a traceback on standard error whose last line
matches CPython's, and the program exits with status 1 (STD-05). A cause
named with `from` is printed first, followed by `The above exception was
the direct cause of the following exception:`, and a context is printed
first, followed by `During handling of the above exception, another
exception occurred:`, as CPython prints them.

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


def try_withdraw(balance: int, amount: int) -> int:
    try:
        return withdraw(balance, amount)
    except ValueError as error:
        print("refused:", error)
        return balance


error: LookupError = KeyError("owner")
print(error, repr(error), type(error).__name__)
print(try_withdraw(10, 3), try_withdraw(10, 30))
```

## What TyPy refuses

`raise` takes an exception or an exception class, so raising a message
fails with E0415:

<!-- typy-expect: rejected E0415 at 1:7 -->
```python
raise "insufficient funds"
```

An `except` clause names classes, so `except "ValueError":` fails with
E0418. A `return`, `break` or `continue` that would leave a `finally`
block drops the exception on its way out, so it fails with E0419, as
CPython 3.14 warns about it. A bare `raise` re-raises the exception being
handled, so it is refused outside an `except` clause with E0416.
`UnicodeDecodeError` and `UnicodeEncodeError` take five arguments that
TyPy does not check, so a program cannot call them (E0417); it can still
catch them.

## Planned

Exception classes of your own come in 0.3 (CLS-03, CLS-13, CLS-14), with
`assert` (SMT-04):

<!-- typy-expect: rejected E0201 at 1:16 -->
```python
class AppError(ValueError):
    pass
```

The `args` attribute comes in 0.4 (EXC-11). Exception groups and
`except*` are planned for after 1.0.
