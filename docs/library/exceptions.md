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
| Classes of your own derived from an exception class, with attributes and methods | CLS-03 |
| `__init__`, `__str__` and `__repr__` defined again in such a class | CLS-13 |
| `super().__init__(...)` in the `__init__` of such a class | CLS-14 |
| `assert test` and `assert test, message`, in every build mode | SMT-04 |

An `except` clause catches an exception of the class it names or of any
class derived from it, so `except LookupError` catches a `KeyError`. The
clauses are tried in order. The name after `as` holds the exception while
the clause runs and is deleted when it ends, as in CPython; with several
classes it is a union of them, such as `ValueError | KeyError`, whose
attributes and methods are those of their nearest common base. An exception no
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

`assert test, message` raises `AssertionError(message)` when the test is
false, and evaluates the message only then. TyPy keeps every `assert` in
every build mode, as CPython does without `-O`. What the test shows holds
after it, so `assert value is not None` lets the code below use `value`
as its base type.

## Exception classes of your own

A class derived from a built-in exception class, or from another such
class defined before it, is an exception class. It holds its base's
attributes first, then its own, and finds a method it does not define in
its nearest base. Calling it makes an exception whose arguments are the
values passed by position, as CPython's `BaseException` keeps them, then
runs `__init__`. `super().__init__(...)` in `__init__` calls the base's
`__init__`, or sets the arguments when the base is built in, and after it
the base's attributes count as assigned.

<!-- typy-expect: accepted -->
```python
class HttpError(Exception):
    def __init__(self, status: int, reason: str) -> None:
        super().__init__(f"{status} {reason}")
        self.status = status


class NotFound(HttpError):
    def __str__(self) -> str:
        return f"not found ({self.status})"


try:
    raise NotFound(404, "missing")
except HttpError as error:
    print(error, repr(error), error.status)
```

Two exceptions compare by identity with `==` and `!=`, as in CPython: an
exception is equal only to itself. An exception class that defines
`__eq__` or `__ne__` is refused (E0201), since TyPy does not call the
special methods of an exception class yet; that is planned for 1.0
(EXC-20).

## Built-in exception classes

| Class | Base |
| --- | --- |
| `BaseException` | `object` |
| `Exception`, `GeneratorExit`, `SystemExit` | `BaseException` |
| `ArithmeticError`, `AssertionError`, `AttributeError`, `EOFError`, `ImportError`, `LookupError`, `NameError` | `Exception` |
| `OSError`, `RuntimeError`, `StopIteration`, `TypeError`, `ValueError` | `Exception` |
| `OverflowError`, `ZeroDivisionError` | `ArithmeticError` |
| `IndexError`, `KeyError` | `LookupError` |
| `NotImplementedError`, `RecursionError` | `RuntimeError` |
| `UnicodeError` | `ValueError` |
| `UnicodeDecodeError`, `UnicodeEncodeError` | `UnicodeError` |
| `BlockingIOError`, `ChildProcessError`, `ConnectionError`, `FileExistsError`, `FileNotFoundError`, `InterruptedError` | `OSError` |
| `IsADirectoryError`, `NotADirectoryError`, `PermissionError`, `ProcessLookupError`, `TimeoutError` | `OSError` |
| `BrokenPipeError`, `ConnectionAbortedError`, `ConnectionRefusedError`, `ConnectionResetError` | `ConnectionError` |

`IOError` and `EnvironmentError` are other names of `OSError`, as in
CPython. An `OSError` that `open` raises reads as CPython's, such as
`[Errno 2] No such file or directory: 'notes.txt'`, and the class is the
one the error number gives, such as `FileNotFoundError`. Reading a file
opened for writing raises `io.UnsupportedOperation: not readable`, which
both `except OSError` and `except ValueError` catch.

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

## Overriding methods

An exception class may define a method of its base again, as any class
may (CLS-04). The exception's own class decides which method runs:

<!-- typy-expect: accepted -->
```python
class AppError(Exception):
    def code(self) -> int:
        return 1


class OtherError(AppError):
    def code(self) -> int:
        return 2


try:
    raise OtherError("busy")
except AppError as error:
    print(error, error.code())
```

## The arguments and the exit code

`error.args` is the tuple of the arguments an exception was built with
(EXC-11), and `stop.code` of a `SystemExit` is `None`, its one argument,
or the tuple of several (EXC-13). Either may hold any value, so TyPy
reads them where a program writes, compares or tests them: `print`,
`str`, `repr`, `bool`, an f-string field, `==` and `!=` with any value, a
condition, `not`, and `len` of `args` (ADR-0030):

<!-- typy-expect: accepted -->
```python
import sys

try:
    raise ValueError("bad value", 3)
except ValueError as error:
    print(error.args, len(error.args), error.args == ("bad value", 3))
try:
    sys.exit(2)
except SystemExit as stop:
    if stop.code == 2:
        print(f"exit code {stop.code!r}")
```

Each is also a value of its own, which you store, pass, index, unpack
and loop over (EXC-15). Its type comes from every place the program and
TyPy itself create an exception of the class or of a class derived from
it: `ValueError("bad", 3)` gives `tuple[str, int]`, and the message of a
`ValueError` TyPy raises, such as the one of `int("x")`, gives
`tuple[str]`. So `args` of a `ValueError` here is
`tuple[str, int] | tuple[str]`, and `error.args[1]` raises `IndexError`
when the exception holds one argument, as CPython does. The five items of
a `UnicodeEncodeError` or `UnicodeDecodeError`, both `ValueError`
classes, join only when the program encodes, decodes or opens a file:

<!-- typy-expect: accepted -->
```python
def parse(text: str) -> int:
    if not text:
        raise ValueError("empty input", 0)
    return int(text)


for text in ["", "x"]:
    try:
        parse(text)
    except ValueError as error:
        message = error.args[0]
        print(message.upper(), len(error.args))
        for item in error.args:
            print(item)
```

A call of an exception class without an `__init__` of the program's
takes `*` of any iterable, and `**` of a dict held in a variable
(EXP-22). The values `*` gives join the other positional values in one
tuple of the first value's type, so `ValueError(*codes)` with a
`list[int]` gives `args` the member `tuple[int, ...]`, and a value of
another type is E0409. An exception class takes no keywords, so a dict
`**` gives raises CPython's `TypeError` when it holds an entry, such as
`ValueError() takes no keyword arguments`. `OSError` and its subclasses
read `errno`, `strerror` and the file names by position, so `*` of a
value only the run knows is E0202 there:

<!-- typy-expect: accepted -->
```python
def check(codes: list[int]) -> None:
    try:
        raise ValueError(*codes)
    except ValueError as error:
        print(len(error.args), error)
        for item in error.args:
            print(item)


check([404, 500])
check([])
```

`code` of a `SystemExit` is `None` for `sys.exit()`, the argument for
one, and the tuple for several. A `KeyError` TyPy raises holds the
missing key, so `args` of a `KeyError` includes `tuple[K]` for the key
type `K` of each dict the program looks up. An exception whose arguments
hold `args` of an exception of a related class, such as
`raise ValueError(error.args)` in an `except ValueError` clause, has a
type without end; it is planned for 1.0 (E0201, EXC-17).

## OS errors

`OSError` and its subclasses built with two to five arguments read them
as CPython does (EXC-14): `errno`, `strerror`, `filename`, `winerror` and
`filename2`. `OSError` itself becomes the subclass for `errno`, so
`OSError(2, "gone")` is a `FileNotFoundError`. The attributes `errno`,
`strerror`, `filename` and `filename2` are `None` when not given, as for
an `OSError` of one argument, and `open` gives them too:

<!-- typy-expect: accepted -->
```python
try:
    raise OSError(2, "No such file or directory", "data.txt")
except FileNotFoundError as error:
    print(error, repr(error))
    print(error.errno, error.strerror, error.filename)
```

The arguments may be of any type, as in CPython (EXC-16):
`OSError("a", "b")` is written `[Errno a] b`, an `OSError` of more than
five arguments reads none of them, and a subclass with an `__init__` of its
own starts with no arguments, which `super().__init__(...)` gives it. The
type of each attribute comes from every place the program and TyPy create
an exception of the class, as the type of `args` does, so `errno` is
`int | None` unless the program builds an `OSError` with another `errno`:

<!-- typy-expect: accepted -->
```python
class ConfigError(OSError):
    def __init__(self, path: str) -> None:
        super().__init__(2, "missing setting", path)


try:
    raise ConfigError("app.toml")
except OSError as error:
    print(error, error.args)
    code = error.errno
    if code is not None:
        print(code + 100, error.filename)
```

A number as the third argument of `OSError` or `BlockingIOError` is
planned for 1.0 (E0201, EXC-18): a `BlockingIOError` reads it as
`characters_written`, and `OSError` becomes one for `errno` 11.

## Planned

The special methods of an exception class, such as `__eq__` and `__lt__`,
called by operators and built-in functions, are planned for 1.0 (EXC-20).
Exception groups and `except*` are planned for after 1.0.
