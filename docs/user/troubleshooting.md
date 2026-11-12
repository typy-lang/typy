# Troubleshooting

Each section is one diagnostic code: what causes it, how to change the
program, and a program that gives the code with its corrected form. A
diagnostic names the rule it enforces, such as `TYP-03`; the
[language guide](guide/index.md) explains each rule. `typy explain E0402`
prints the section of a code in the terminal.

## E0101: source file larger than 16 MiB

The main file is over 16 MiB, so TyPy refuses it before reading it (LEX-09).
Split the program, or move large data out of the source.

## E0102: source file is not valid UTF-8

The file holds bytes that are not UTF-8 (LEX-01). Save the file as UTF-8.

## E0103: Python syntax error

The source is not valid Python for every CPython version from the
program's floor up, 3.12 by default (STD-01), or `return`, `break` or
`continue` is out of place. Fix the syntax CPython would also report, or
write the construct in a form CPython 3.12 accepts.

This program gives E0103:

<!-- typy-expect: rejected E0103 at 2:1 -->
```python
print("total"
```

This one is correct:

<!-- typy-expect: accepted -->
```python
print("total")
```

## E0104: imaginary literal

A literal such as `2j` makes a `complex`, a type TyPy does not have
(LEX-08). Use two `float` values for the real and imaginary parts.

This program gives E0104:

<!-- typy-expect: rejected E0104 at 1:5 -->
```python
z = 2j
print(z)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
real, imaginary = 0.0, 2.0
print(real, imaginary)
```

## E0105: source file cannot be read

The file named on the command line does not exist or cannot be opened.
Check the path and its permissions.

## E0106: output file cannot be written

TyPy cannot write the executable or the stage output. Check that the
directory exists and can be written, or give another path with `-o`.

## E0107: `typy.toml` is not valid

The project's `typy.toml` cannot be used: it is not valid TOML, it has a
setting TyPy does not know, a value has the wrong type, such as
`python-floor = 3.15` where the string `"3.15"` is meant, or a module path
is not a directory under the project root (CLI-07, CLI-12). The diagnostic
points at the line. TyPy exits with status 2 before it reads any module.
Correct the line; the settings are listed in the
[command line](guide/command-line.md) page.

This `typy.toml` gives E0107, since the floor is a number:

```toml
python-floor = 3.15
```

This one is correct:

```toml
python-floor = "3.15"
```

## E0201: feature planned for a later TyPy release

The program uses valid Python that a later TyPy release accepts, such as
`try`, `import` or inheritance. The message names the release and the rule.
Write the code without the feature, for example an f-string instead of `%`
formatting, or wait for that release.

This program gives E0201:

<!-- typy-expect: rejected E0201 at 5:14 -->
```python
class Shape:
    pass


class Square(Shape):
    pass
```

This one is correct:

<!-- typy-expect: accepted -->
```python
class Square:
    pass


print(Square.__name__)
```

## E0202: feature TyPy cannot compile with CPython's meaning

The program uses valid Python that TyPy refuses because it cannot give it
CPython's meaning (STD-04), such as `global`, a class defined inside a
function, or a builtin TyPy does not provide. Restructure the code: pass
values as arguments and return them, and define classes and functions at
the top of the module.

This program gives E0202:

<!-- typy-expect: rejected E0202 at 1:7 -->
```python
print(eval("1 + 2"))
```

This one is correct:

<!-- typy-expect: accepted -->
```python
print(1 + 2)
```

## E0203: parameter without a type annotation

Every parameter except `self` needs an annotation (TYP-01). Write the
type, such as `def area(width: float) -> float:`.

This program gives E0203:

<!-- typy-expect: rejected E0203 at 1:12 -->
```python
def double(n) -> int:
    return n * 2


print(double(2))
```

This one is correct:

<!-- typy-expect: accepted -->
```python
def double(n: int) -> int:
    return n * 2


print(double(2))
```

## E0204: function without a return annotation

Every function needs a return annotation (TYP-01). Write `-> None` when it
returns nothing. `__init__` may leave it out.

This program gives E0204:

<!-- typy-expect: rejected E0204 at 1:5 -->
```python
def greet(name: str):
    print("hello", name)


greet("Ada")
```

This one is correct:

<!-- typy-expect: accepted -->
```python
def greet(name: str) -> None:
    print("hello", name)


greet("Ada")
```

## E0205: name is not defined

The name is not a variable, function, class or builtin of the program, or
an annotation does not name a type. Check the spelling, and define the
name before the line that uses it.

This program gives E0205:

<!-- typy-expect: rejected E0205 at 2:7 -->
```python
total = 3
print(totl)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
total = 3
print(total)
```

## E0206: import of a module TyPy does not provide

The program imports a module that neither the program nor TyPy provides
(LIB-08). A module of the program is a `.py` file, or a package directory
with an `__init__.py`, under the main file's directory: `import tools.text`
reads `tools/__init__.py` and `tools/text.py`. A relative import such as
`from . import helper` works only inside a package, as in CPython. For the
standard library, use the modules the message lists, or write the code
without the module.

This program gives E0206:

<!-- typy-expect: rejected E0206 at 1:1 -->
```python
import json
```

This one is correct:

<!-- typy-expect: accepted -->
```python
values = [1, 2]
print(values)
```

## E0301: `is` used on values other than `None`, `bool` values or class instances

`is` compares identity, which TyPy defines only against `None`, between
`bool` and `bool | None` values, and between instances or type objects
(EXP-08). Compare
other values with `==`: CPython's identity of numbers and strings depends
on its caches, so the same program could print different results.

This program gives E0301:

<!-- typy-expect: rejected E0301 at 2:16 -->
```python
count = 3
print(count is 3)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
count = 3
print(count == 3)
```

## E0302: `**` whose result in CPython could have another type

CPython's result of `x ** y` would be a `float` or a `complex` for some
values, such as a negative exponent (EXP-04). Write the exponent as a
non-negative integer literal, or use `float` operands.

This program gives E0302:

<!-- typy-expect: rejected E0302 at 2:20 -->
```python
def power(base: int, exponent: int) -> int:
    return base ** exponent


print(power(2, 3))
```

This one is correct:

<!-- typy-expect: accepted -->
```python
def cube(base: int) -> int:
    return base ** 3


print(cube(2))
```

## E0303: `and` or `or` with operands of different types

`and` and `or` return one of their operands, which must have one type
(EXP-09). Make both operands the same type, or use an `if` statement.

This program gives E0303:

<!-- typy-expect: rejected E0303 at 2:15 -->
```python
name = ""
print(name or 0)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
name = ""
print(name or "anonymous")
```

## E0304: truth test on a value of an unsupported type

The value cannot be tested for truth (EXP-10). Compare it explicitly, such
as `count > 0`.

This program gives E0304:

<!-- typy-expect: rejected E0304 at 5:8 -->
```python
from typing import Iterable


def show(values: Iterable[int]) -> None:
    if values:
        print("some")
```

This one is correct:

<!-- typy-expect: accepted -->
```python
def show(values: list[int]) -> None:
    if len(values) > 0:
        print("some")


show([1])
```

## E0305: conditional expression with branches of different types

`x if c else y` gives one of two values, which must have one type
(EXP-11). Make both branches the same type.

This program gives E0305:

<!-- typy-expect: rejected E0305 at 2:32 -->
```python
count = 2
print("many" if count > 1 else count)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
count = 2
print("many" if count > 1 else str(count))
```

## E0306: variable read before it is assigned on every path

A variable, a function or a class is read where it may not be assigned
yet, which CPython would report as `UnboundLocalError` or `NameError`
(TYP-12). Assign the variable on every path, move a definition above the
statement that uses it, or quote a class name in an annotation, such as
`"Node"`.

This program gives E0306:

<!-- typy-expect: rejected E0306 at 4:7 -->
```python
for number in range(3):
    if number > 1:
        last = number
print(last)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
last = -1
for number in range(3):
    if number > 1:
        last = number
print(last)
```

## E0307: tuple indexed other than by an integer literal inside it

Each item of a fixed tuple has its own type, so its position must be a
literal inside the tuple (DAT-08). Use a literal index, unpack the tuple,
or use `tuple[T, ...]` when every item has one type.

This program gives E0307:

<!-- typy-expect: rejected E0307 at 3:12 -->
```python
pair = (1, "one")
position = 0
print(pair[position])
```

This one is correct:

<!-- typy-expect: accepted -->
```python
pair = (1, "one")
number, name = pair
print(number, name)
```

## E0308: unpacking into a different number of targets

The value has a different number of items than there are targets
(SMT-02). Use as many targets as the tuple has items.

This program gives E0308:

<!-- typy-expect: rejected E0308 at 1:8 -->
```python
x, y = (1, 2, 3)
print(x, y)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
x, y, z = (1, 2, 3)
print(x, y, z)
```

## E0309: instance used before `__init__` assigns every attribute

In `__init__`, `self` was passed on, a method was called on it, an
attribute was read before it was assigned, or `__init__` can end without
assigning an attribute (CLS-01). Assign every attribute on every path
first, then use `self`.

This program gives E0309:

<!-- typy-expect: rejected E0309 at 3:15 -->
```python
class Point:
    def __init__(self) -> None:
        print(self.x)
        self.x = 0


Point()
```

This one is correct:

<!-- typy-expect: accepted -->
```python
class Point:
    def __init__(self) -> None:
        self.x = 0
        print(self.x)


Point()
```

## E0310: attribute whose type is not known yet

An attribute is read where its type is not known yet, because the
`__init__` that assigns it is checked later (CLS-01). Declare the attribute
in the class body, such as `value: int`.

This program gives E0310:

<!-- typy-expect: rejected E0310 at 4:25 -->
```python
class Pair:
    def __init__(self, other: "Pair | None") -> None:
        if other is not None:
            print(other.left)
        self.left = 1
```

This one is correct:

<!-- typy-expect: accepted -->
```python
class Pair:
    left: int

    def __init__(self, other: "Pair | None") -> None:
        if other is not None:
            print(other.left)
        self.left = 1


Pair(Pair(None))
```

## E0401: operator not defined for these operand types

The operator, method or use does not apply to these types (EXP-02), such
as `1 + "a"`, or an optional value used before a test shows it is not
`None` (TYP-13). Convert one operand, such as `str(1) + "a"`, or test the
optional value with `if value is not None:` first.

This program gives E0401:

<!-- typy-expect: rejected E0401 at 1:7 -->
```python
print(1 + "a")
```

This one is correct:

<!-- typy-expect: accepted -->
```python
print(str(1) + "a")
```

## E0402: assignment changes the type of a variable

A variable or attribute keeps the type of its first value or annotation
(TYP-03). Use a new variable for a value of another type. An `int` where a
`float` is declared is also refused (TYP-09): write `1.0`.

This program gives E0402:

<!-- typy-expect: rejected E0402 at 2:9 -->
```python
count = 0
count = "none"
print(count)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
count = 0
label = "none"
print(count, label)
```

## E0403: `Any`, `cast` or an untyped value

`typing.Any`, `typing.cast` and `object` annotations would leave a value
without one type (TYP-10). Annotate the type the value holds.

This program gives E0403:

<!-- typy-expect: rejected E0403 at 1:8 -->
```python
value: object = 1
print(value)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
value: int = 1
print(value)
```

## E0404: argument of the wrong type

An argument does not match its parameter, an argument is missing or extra,
or a keyword names no parameter (TYP-01). Pass a value of the parameter's
type.

This program gives E0404:

<!-- typy-expect: rejected E0404 at 5:14 -->
```python
def double(n: int) -> int:
    return n * 2


print(double("2"))
```

This one is correct:

<!-- typy-expect: accepted -->
```python
def double(n: int) -> int:
    return n * 2


print(double(int("2")))
```

## E0405: returned value of the wrong type

The returned value does not match the return annotation, or a function
that returns a value can reach its end without returning (TYP-01). Return
a value of the declared type on every path.

This program gives E0405:

<!-- typy-expect: rejected E0405 at 2:12 -->
```python
def half(n: int) -> int:
    return n / 2


print(half(4))
```

This one is correct:

<!-- typy-expect: accepted -->
```python
def half(n: int) -> int:
    return n // 2


print(half(4))
```

## E0406: arithmetic on `bool`

`True + 1` is valid Python, but TyPy keeps `bool` and `int` apart
(DAT-03). Convert with `int(flag)` first.

This program gives E0406:

<!-- typy-expect: rejected E0406 at 2:7 -->
```python
done = True
print(done + 1)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
done = True
print(int(done) + 1)
```

## E0407: comparison between unsupported types

The two values cannot be compared, such as a `str` with an `int`, or
instances ordered with `<` (EXP-06). Compare values of one type.

This program gives E0407:

<!-- typy-expect: rejected E0407 at 1:14 -->
```python
print("10" < 9)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
print(int("10") < 9)
```

## E0408: annotation without its item types

A `tuple`, `list`, `set` or `dict` annotation names no item types (TYP-06).
Write them, such as `list[int]` or `dict[str, float]`.

This program gives E0408:

<!-- typy-expect: rejected E0408 at 1:19 -->
```python
def total(values: list) -> int:
    return sum(values)


print(total([1, 2]))
```

This one is correct:

<!-- typy-expect: accepted -->
```python
def total(values: list[int]) -> int:
    return sum(values)


print(total([1, 2]))
```

## E0409: list, set or dict display with items of different types

Every item of a display has one type (DAT-07, DAT-09, DAT-10). Make the
items one type, or use a tuple for values of different types.

This program gives E0409:

<!-- typy-expect: rejected E0409 at 1:11 -->
```python
row = [1, "two"]
print(row)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
row = (1, "two")
print(row)
```

## E0410: empty collection without an annotation

`[]`, `{}`, `set()` and `dict()` need their item types from an annotation
or from where the value goes (TYP-05). Annotate the variable, such as
`names: list[str] = []`.

This program gives E0410:

<!-- typy-expect: rejected E0410 at 1:9 -->
```python
names = []
names.append("Ada")
print(names)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
names: list[str] = []
names.append("Ada")
print(names)
```

## E0411: `for` over a value that cannot be iterated

The loop's source cannot be iterated, or its items would not have one
type, as in a tuple of mixed items (CST-03). Loop over a list or a
`tuple[T, ...]` of one item type.

This program gives E0411:

<!-- typy-expect: rejected E0411 at 1:14 -->
```python
for digit in 123:
    print(digit)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
for digit in str(123):
    print(digit)
```

## E0412: method a built-in type does not have

The value's type has no such method or attribute in TyPy (LIB-03). Check
the method's name and the value's type.

This program gives E0412:

<!-- typy-expect: rejected E0412 at 1:13 -->
```python
print("a,b".explode(","))
```

This one is correct:

<!-- typy-expect: accepted -->
```python
print("a,b".split(","))
```

## E0413: set item or dict key of a type without a hash

A list, a set, a dict or a tuple that holds one cannot be a set item or a
dict key (DAT-09, DAT-10). Use a tuple, a `str` or a number as the key.

This program gives E0413:

<!-- typy-expect: rejected E0413 at 1:8 -->
```python
seen = {[1, 2]}
print(seen)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
seen = {(1, 2)}
print(seen)
```

## E0414: attribute or method a class does not have

The class has no attribute or method of that name, and an attribute
cannot be added outside `__init__` (CLS-02). Declare the attribute in the
class body or assign it in `__init__`.

This program gives E0414:

<!-- typy-expect: rejected E0414 at 7:9 -->
```python
class Point:
    def __init__(self) -> None:
        self.x = 0


p = Point()
print(p.y)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
class Point:
    def __init__(self) -> None:
        self.x = 0


p = Point()
print(p.x)
```

## E0415: `raise` of a value that is not an exception

`raise` and `from` take an exception, such as `ValueError("bad input")`,
or an exception class, which is called with no arguments (SMT-06). Raise
an exception, not a message: `raise ValueError("bad input")` rather than
`raise "bad input"`.

This program gives E0415:

<!-- typy-expect: rejected E0415 at 1:7 -->
```python
raise "failed"
```

This one is correct:

<!-- typy-expect: accepted -->
```python
try:
    raise ValueError("failed")
except ValueError as error:
    print(error)
```

## E0416: bare `raise` outside an `except` clause

A `raise` with nothing after it raises the exception being handled again,
so it belongs inside an `except` clause (EXC-07). Elsewhere, name the
exception to raise.

This program gives E0416:

<!-- typy-expect: rejected E0416 at 2:5 -->
```python
def fail() -> None:
    raise
```

This one is correct:

<!-- typy-expect: accepted -->
```python
def fail() -> None:
    raise RuntimeError("failed")


try:
    fail()
except RuntimeError as error:
    print(error)
```

## E0417: exception class a program cannot build

`UnicodeDecodeError` and `UnicodeEncodeError` take five arguments that
TyPy does not check, so a program cannot call them (EXC-12). It can still
catch them. Raise `ValueError` or a class of your own instead.

This program gives E0417:

<!-- typy-expect: rejected E0417 at 1:9 -->
```python
error = UnicodeDecodeError("utf-8", b"", 0, 1, "bad")
```

This one is correct:

<!-- typy-expect: accepted -->
```python
error = ValueError("bad")
print(error)
```

## E0418: `except` clause that names something other than classes

An `except` clause names one exception class, such as `except KeyError:`,
or a tuple of them, such as `except (KeyError, IndexError):` (EXC-09). An
instance, a string or a variable that holds a class is refused. Name the
class itself.

This program gives E0418:

<!-- typy-expect: rejected E0418 at 3:8 -->
```python
try:
    print(1 // 0)
except "zero":
    pass
```

This one is correct:

<!-- typy-expect: accepted -->
```python
try:
    print(1 // 0)
except ZeroDivisionError:
    print("zero")
```

## E0419: `return`, `break` or `continue` that leaves a `finally` block

Leaving a `finally` block with `return`, `break` or `continue` drops an
exception that is on its way out, so TyPy refuses it, as CPython 3.14
warns about it (EXC-06). Move the statement after the `try` statement. A
`break` or `continue` of a loop inside the `finally` block is accepted.

This program gives E0419:

<!-- typy-expect: rejected E0419 at 5:9 -->
```python
def first() -> int:
    try:
        return 1
    finally:
        return 2


print(first())
```

This one is correct:

<!-- typy-expect: accepted -->
```python
def first() -> int:
    try:
        return 1
    finally:
        print("done")


print(first())
```

## E0420: `with` on a value without `__enter__` and `__exit__`

A `with` statement needs a context manager: an instance of a class that
defines `__enter__` and `__exit__` (CST-05). Define both in the class, or
use `try` and `finally` for a value that has none.

This program gives E0420:

<!-- typy-expect: rejected E0420 at 5:6 -->
```python
class Plain:
    pass


with Plain():
    pass
```

This one is correct:

<!-- typy-expect: accepted -->
```python
class Opened:
    def __enter__(self) -> "Opened":
        print("enter")
        return self

    def __exit__(self, kind, value, trace) -> None:
        print("exit")


with Opened():
    pass
```

## E0421: `del` of something other than an item, a slice or a key

`del` removes a list item, a list slice or a dict key (SMT-05). A
variable or an attribute stays bound once assigned in TyPy, and a tuple,
a `str` or `bytes` cannot change. Assign a new value instead, or keep the
values in a list or a dict.

This program gives E0421:

<!-- typy-expect: rejected E0421 at 2:5 -->
```python
count = 1
del count
```

This one is correct:

<!-- typy-expect: accepted -->
```python
counts = [1, 2]
del counts[0]
print(counts)
```

## E0422: circular import at module level

Two or more modules import each other at module level (MOD-05), so one of
them would run while another is half initialised. The message shows each
import of the cycle. Move one import into the function that uses the
module: an import inside a function runs when the function is called, and
makes no cycle.

These two modules give E0422, since each imports the other while it
starts:

```text
# main.py
import shapes

# shapes.py
import main
```

Moving one import into the function that uses it is correct:

```text
# main.py
import shapes

# shapes.py
def draw() -> None:
    import main
```

## E0423: `global` of a name the module does not bind

A module variable keeps one type, which the module's own code gives it
(SMT-08), so a function cannot create one with `global`. Assign the
variable at module level, or declare its type there without a value,
such as `total: int`.

This program gives E0423:

<!-- typy-expect: rejected E0423 at 2:12 -->
```python
def reset() -> None:
    global total
    total = 0
```

This one is correct:

<!-- typy-expect: accepted -->
```python
total = 5


def reset() -> None:
    global total
    total = 0


reset()
print(total)
```

## E0424: an `__init__` reads a module variable before its type is known

Each `__init__` is checked before the module's code, because the module
reads the attributes it assigns. A module variable whose value needs
those attributes has no type yet when the `__init__` reads it. Declare
its type at module level, such as `LIMIT: int = c.v`.

This program gives E0424:

<!-- typy-expect: rejected E0424 at 8:24 -->
```python
class Config:
    def __init__(self) -> None:
        self.limit = 10


class Gauge:
    def __init__(self) -> None:
        self.ceiling = LIMIT


LIMIT = Config().limit
print(Gauge().ceiling)
```

This one is correct:

<!-- typy-expect: accepted -->
```python
class Config:
    def __init__(self) -> None:
        self.limit = 10


class Gauge:
    def __init__(self) -> None:
        self.ceiling = LIMIT


LIMIT: int = Config().limit
print(Gauge().ceiling)
```

## E0425: import of a name its module does not define

`from m import x` and `m.x` name something the module `m` defines at
module level: a function, a class, a variable it assigns, or one of its
submodules (SMT-07). Check the spelling, and that `m` binds the name at
module level rather than inside a function.

With `shapes.py` holding `def area(width: float) -> float`, this
`main.py` gives E0425, since `shapes` has no `volume`:

```text
from shapes import volume
```

Importing a name the module defines is correct:

```text
from shapes import area
```

## E0426: import inside a function of a module that imports this one

TyPy checks each module after the modules it imports at module level.
When module `a` imports `b` at module level and a function of `b` imports
`a`, `b` is checked before `a`, so its function cannot see `a`'s names
yet; this is planned for 0.4 (MOD-05). Move the code both modules share
into a third module that each imports at module level.

These two modules give E0426, since `main` imports `shapes` at module
level and a function of `shapes` imports `main`:

```text
# main.py
import shapes

# shapes.py
def draw() -> None:
    import main
    main.show()
```

Moving the shared function into a third module that both import is
correct:

```text
# main.py
import shapes
import display

# shapes.py
import display
```

## E0701: C toolchain program not found

TyPy links programs with the system C toolchain, which is not installed
(A-01). Install it, such as `sudo apt install build-essential`.

## E0702: link failed

The C toolchain could not link the program (A-01). The message shows the
linker's output; check that the toolchain works with `cc --version`.

## E0703: runtime archive missing, or from another TyPy version

The runtime library TyPy links into programs is not beside the `typy`
command, or comes from another version. Unpack the release archive again
and run its own `bin/typy`.

## E9001: internal compiler error

TyPy itself failed. This is a defect in TyPy: please report it with the
program that caused it.

## W0101: invalid escape sequence

A string or bytes literal holds a backslash before a character that does
not start an escape, such as `"\d"`, or an octal escape above `\377`.
CPython keeps the backslash and warns while compiling, and so does TyPy
(DIA-08). Write the backslash twice, `"\\d"`, which keeps the same value
and which the diagnostic shows, or use a raw string, `r"\d"`.

This program gives W0101 and still runs:

<!-- typy-expect: accepted -->
```python
print("\d+ digits")
```

This one is correct:

<!-- typy-expect: accepted -->
```python
print("\\d+ digits")
```

## W0301: assertion that is always true

`assert (test, "message")` asserts a tuple with two items, which is always
true, so the assertion never fails (DIA-08). Remove the parentheses:
`assert test, "message"`.

This program gives W0301 and still runs:

<!-- typy-expect: accepted -->
```python
count = 0
assert (count == 0, "count starts at zero")
```

This one is correct:

<!-- typy-expect: accepted -->
```python
count = 0
assert count == 0, "count starts at zero"
```
