# Troubleshooting

Each section is one diagnostic code: what causes it and how to change the
program. A diagnostic names the rule it enforces, such as `TYP-03`; the
[language guide](guide/index.md) explains each rule.

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

## E0104: imaginary literal

A literal such as `2j` makes a `complex`, a type TyPy does not have
(LEX-08). Use two `float` values for the real and imaginary parts.

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

## E0201: feature planned for a later TyPy release

The program uses valid Python that a later TyPy release accepts, such as
`try`, `import` or inheritance. The message names the release and the rule.
Write the code without the feature, for example an f-string instead of `%`
formatting, or wait for that release.

## E0202: feature TyPy cannot compile with CPython's meaning

The program uses valid Python that TyPy refuses because it cannot give it
CPython's meaning (STD-04), such as `global`, a class defined inside a
function, or a builtin TyPy does not provide. Restructure the code: pass
values as arguments and return them, and define classes and functions at
the top of the module.

## E0203: parameter without a type annotation

Every parameter except `self` needs an annotation (TYP-01). Write the
type, such as `def area(width: float) -> float:`.

## E0204: function without a return annotation

Every function needs a return annotation (TYP-01). Write `-> None` when it
returns nothing. `__init__` may leave it out.

## E0205: name is not defined

The name is not a variable, function, class or builtin of the program, or
an annotation does not name a type. Check the spelling, and define the
name before the line that uses it.

## E0206: import of a module TyPy does not provide

The program imports a module that neither the program nor TyPy provides
(LIB-08). A module of the program is a `.py` file, or a package directory
with an `__init__.py`, under the main file's directory: `import tools.text`
reads `tools/__init__.py` and `tools/text.py`. A relative import such as
`from . import helper` works only inside a package, as in CPython. For the
standard library, use the modules the message lists, or write the code
without the module.

## E0301: `is` used on values other than `None`, `bool` values or class instances

`is` compares identity, which TyPy defines only against `None`, between
`bool` and `bool | None` values, and between instances or type objects
(EXP-08). Compare
other values with `==`: CPython's identity of numbers and strings depends
on its caches, so the same program could print different results.

## E0302: `**` whose result in CPython could have another type

CPython's result of `x ** y` would be a `float` or a `complex` for some
values, such as a negative exponent (EXP-04). Write the exponent as a
non-negative integer literal, or use `float` operands.

## E0303: `and` or `or` with operands of different types

`and` and `or` return one of their operands, which must have one type
(EXP-09). Make both operands the same type, or use an `if` statement.

## E0304: truth test on a value of an unsupported type

The value cannot be tested for truth (EXP-10). Compare it explicitly, such
as `count > 0`.

## E0305: conditional expression with branches of different types

`x if c else y` gives one of two values, which must have one type
(EXP-11). Make both branches the same type.

## E0306: variable read before it is assigned on every path

A variable, a function or a class is read where it may not be assigned
yet, which CPython would report as `UnboundLocalError` or `NameError`
(TYP-12). Assign the variable on every path, move a definition above the
statement that uses it, or quote a class name in an annotation, such as
`"Node"`.

## E0307: tuple indexed other than by an integer literal inside it

Each item of a fixed tuple has its own type, so its position must be a
literal inside the tuple (DAT-08). Use a literal index, unpack the tuple,
or use `tuple[T, ...]` when every item has one type.

## E0308: unpacking into a different number of targets

The value has a different number of items than there are targets
(SMT-02). Use as many targets as the tuple has items.

## E0309: instance used before `__init__` assigns every attribute

In `__init__`, `self` was passed on, a method was called on it, an
attribute was read before it was assigned, or `__init__` can end without
assigning an attribute (CLS-01). Assign every attribute on every path
first, then use `self`.

## E0310: attribute whose type is not known yet

An attribute is read where its type is not known yet, because the
`__init__` that assigns it is checked later (CLS-01). Declare the attribute
in the class body, such as `value: int`.

## E0401: operator not defined for these operand types

The operator, method or use does not apply to these types (EXP-02), such
as `1 + "a"`, or an optional value used before a test shows it is not
`None` (TYP-13). Convert one operand, such as `str(1) + "a"`, or test the
optional value with `if value is not None:` first.

## E0402: assignment changes the type of a variable

A variable or attribute keeps the type of its first value or annotation
(TYP-03). Use a new variable for a value of another type. An `int` where a
`float` is declared is also refused (TYP-09): write `1.0`.

## E0403: `Any`, `cast` or an untyped value

`typing.Any`, `typing.cast` and `object` annotations would leave a value
without one type (TYP-10). Annotate the type the value holds.

## E0404: argument of the wrong type

An argument does not match its parameter, an argument is missing or extra,
or a keyword names no parameter (TYP-01). Pass a value of the parameter's
type.

## E0405: returned value of the wrong type

The returned value does not match the return annotation, or a function
that returns a value can reach its end without returning (TYP-01). Return
a value of the declared type on every path.

## E0406: arithmetic on `bool`

`True + 1` is valid Python, but TyPy keeps `bool` and `int` apart
(DAT-03). Convert with `int(flag)` first.

## E0407: comparison between unsupported types

The two values cannot be compared, such as a `str` with an `int`, or
instances ordered with `<` (EXP-06). Compare values of one type.

## E0408: annotation without its item types

A `tuple`, `list`, `set` or `dict` annotation names no item types (TYP-06).
Write them, such as `list[int]` or `dict[str, float]`.

## E0409: list, set or dict display with items of different types

Every item of a display has one type (DAT-07, DAT-09, DAT-10). Make the
items one type, or use a tuple for values of different types.

## E0410: empty collection without an annotation

`[]`, `{}`, `set()` and `dict()` need their item types from an annotation
or from where the value goes (TYP-05). Annotate the variable, such as
`names: list[str] = []`.

## E0411: `for` over a value that cannot be iterated

The loop's source cannot be iterated, or its items would not have one
type, as in a tuple of mixed items (CST-03). Loop over a list or a
`tuple[T, ...]` of one item type.

## E0412: method a built-in type does not have

The value's type has no such method or attribute in TyPy (LIB-03). Check
the method's name and the value's type.

## E0413: set item or dict key of a type without a hash

A list, a set, a dict or a tuple that holds one cannot be a set item or a
dict key (DAT-09, DAT-10). Use a tuple, a `str` or a number as the key.

## E0414: attribute or method a class does not have

The class has no attribute or method of that name, and an attribute
cannot be added outside `__init__` (CLS-02). Declare the attribute in the
class body or assign it in `__init__`.

## E0415: `raise` of a value that is not an exception

`raise` and `from` take an exception, such as `ValueError("bad input")`,
or an exception class, which is called with no arguments (SMT-06). Raise
an exception, not a message: `raise ValueError("bad input")` rather than
`raise "bad input"`.

## E0416: bare `raise` outside an `except` clause

A `raise` with nothing after it raises the exception being handled again,
so it belongs inside an `except` clause (EXC-07). Elsewhere, name the
exception to raise.

## E0417: exception class a program cannot build

`UnicodeDecodeError` and `UnicodeEncodeError` take five arguments that
TyPy does not check, so a program cannot call them (EXC-12). It can still
catch them. Raise `ValueError` or a class of your own instead.

## E0418: `except` clause that names something other than classes

An `except` clause names one exception class, such as `except KeyError:`,
or a tuple of them, such as `except (KeyError, IndexError):` (EXC-09). An
instance, a string or a variable that holds a class is refused. Name the
class itself.

## E0419: `return`, `break` or `continue` that leaves a `finally` block

Leaving a `finally` block with `return`, `break` or `continue` drops an
exception that is on its way out, so TyPy refuses it, as CPython 3.14
warns about it (EXC-06). Move the statement after the `try` statement. A
`break` or `continue` of a loop inside the `finally` block is accepted.

## E0420: `with` on a value without `__enter__` and `__exit__`

A `with` statement needs a context manager: an instance of a class that
defines `__enter__` and `__exit__` (CST-05). Define both in the class, or
use `try` and `finally` for a value that has none.

## E0421: `del` of something other than an item, a slice or a key

`del` removes a list item, a list slice or a dict key (SMT-05). A
variable or an attribute stays bound once assigned in TyPy, and a tuple,
a `str` or `bytes` cannot change. Assign a new value instead, or keep the
values in a list or a dict.

## E0422: circular import at module level

Two or more modules import each other at module level (MOD-05), so one of
them would run while another is half initialised. The message shows each
import of the cycle. Move one import into the function that uses the
module: an import inside a function runs when the function is called, and
makes no cycle.

## E0423: `global` of a name the module does not bind

A module variable keeps one type, which the module's own code gives it
(SMT-08), so a function cannot create one with `global`. Assign the
variable at module level, or declare its type there without a value,
such as `total: int`.

## E0424: an `__init__` reads a module variable before its type is known

Each `__init__` is checked before the module's code, because the module
reads the attributes it assigns. A module variable whose value needs
those attributes has no type yet when the `__init__` reads it. Declare
its type at module level, such as `LIMIT: int = c.v`.

## E0425: import of a name its module does not define

`from m import x` and `m.x` name something the module `m` defines at
module level: a function, a class, a variable it assigns, or one of its
submodules (SMT-07). Check the spelling, and that `m` binds the name at
module level rather than inside a function.

## E0426: import inside a function of a module that imports this one

TyPy checks each module after the modules it imports at module level.
When module `a` imports `b` at module level and a function of `b` imports
`a`, `b` is checked before `a`, so its function cannot see `a`'s names
yet; this is planned for 0.4 (MOD-05). Move the code both modules share
into a third module that each imports at module level.

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

## W0301: assertion that is always true

`assert (test, "message")` asserts a tuple with two items, which is always
true, so the assertion never fails (DIA-08). Remove the parentheses:
`assert test, "message"`.
