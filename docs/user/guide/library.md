# Built-in functions and library

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `print` with `sep` and `end`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `bool`, `repr`, `ord` and `chr` | LIB-01 |
| `list`, `dict`, `set`, `tuple`, `sorted`, `reversed`, `enumerate`, `zip`, `sum`, `any`, `all`, `round` and `type` | LIB-02, CLS-12 |
| The methods of `str`, `bytes`, `list`, `tuple`, `dict` and `set`, with CPython's results | LIB-03, DAT-15 |
| `format % values`, printf-style formatting of a string literal | LIB-10 |
| The `math` module: its constants and its functions of numbers | LIB-05 |
| `sys.argv`, `sys.exit`, `sys.stdout` and `sys.stderr`, `print` with `file` and `flush`, and `input` | LIB-04 |
| `open` for text files in UTF-8, with `read`, `readline`, `readlines`, `write`, `writelines`, `flush`, `close`, iteration and `with` | LIB-04 |

`str` has `find`, `index`, `count`, `split`, `join`, `strip`, `replace`,
`startswith`, `upper`, `lower`, `title`, `center`, `zfill`, `encode`, the
`is` tests and the rest of the methods that read or rebuild text; `bytes`
has the same with `decode` and `hex`. Lists have `append`, `pop`,
`insert`, `extend`, `remove`, `index`, `count`, `clear`, `copy`,
`reverse` and `sort`; dicts `get`, `setdefault`, `pop`, `popitem`, `keys`,
`values`, `items`, `update`, `copy` and `clear`; sets `add`, `remove`,
`discard`, `pop` and the methods that combine sets.

`iter` and `next` are
described in
[Compound statements](compound-statements.md#iterators).

## Example

<!-- typy-expect: accepted -->
```python
words = "the cat sat on the mat".split()
lengths = [len(word) for word in words]
has_cat = any([word == "cat" for word in words])
print(sorted(set(words)), sum(lengths), max(lengths), has_cat)
print("-".join(words), list(zip(words, lengths))[:2])
print(round(2.675, 2), abs(-7), "Title Case".swapcase(), b"abc".hex())
print("  x ".strip(), "a,b".split(","))
```

## Formatting with `%`

`format % values` formats a string literal as CPython does (LIB-10). The
right operand is a tuple of values, one value that is not a tuple, or a
dict for conversions with a key, such as `%(name)s`. Every conversion
CPython has works, with its flags, width and precision, `*` for a width
or a precision taken from the values, and `%%` for a `%`:

<!-- typy-expect: accepted -->
```python
name, count, ratio = "Ada", 3, 2 / 3
print("%s has %d items, %.1f%% done" % (name, count, ratio * 100))
print("%-6s|%05.2f|%#x|%c" % ("id", ratio, 255, 65))
print("%(user)s plays for %(team)s" % {"user": "Ada", "team": "blue"})
print("%*d" % (6, count))
```

TyPy reads the format when it checks the program, so a format that does
not fit its values is E0427, with the message CPython's `TypeError` or
`ValueError` would give, in CPython 3.15's words, such as
`not enough arguments for format string (got 1)`.
A format that is not a literal, and a `tuple[T, ...]` of values, are
planned for 0.4 (LIB-11); write an f-string for those:

<!-- typy-expect: rejected E0427 at 1:25 -->
```python
print("%d of %d done" % (3,))
```

## The math module

`import math` and `from math import sqrt` give the `math` module of
CPython (LIB-05): `pi`, `e`, `tau`, `inf` and `nan`, and its functions of
numbers, with CPython's results to the last bit and its errors, such as
`ValueError: expected a nonnegative input, got -1.0` for `math.sqrt(-1)`.
A function that takes a real number takes an `int`, a `bool` or a
`float`, as in CPython:

<!-- typy-expect: accepted -->
```python
import math
from math import sqrt, gcd

print(sqrt(2), math.floor(-2.5), math.log(8, 2), math.hypot(3, 4))
print(math.factorial(20), gcd(12, 18, 30), math.isclose(0.1 + 0.2, 0.3))
print(math.frexp(8.0), math.degrees(math.pi), math.log10(10**400))
```

A function the module gained after the program's Python floor is E0425
at that floor: `math.fma` needs `python-floor = "3.13"`, and `fmax`,
`fmin`, `signbit`, `isnormal` and `issubnormal` need `"3.15"`.
`fsum`, `prod`, `sumprod`, `dist`, `gamma` and `lgamma` are planned for
0.4 (LIB-12). A function of `math` is not a value yet: `f = math.sqrt`
is planned for 1.0 (FUN-05).

## The sys module

`import sys` gives four names of CPython's `sys` module (LIB-04):

| Name | What it is |
| --- | --- |
| `sys.argv` | A `list[str]`: the path of the executable, then each argument |
| `sys.exit(code)` | Raises `SystemExit(code)`; `code` may be left out |
| `sys.stdout`, `sys.stderr` | Text files with `write(text)`, which returns the number of characters, and `flush()` |

An argument that is not valid UTF-8 keeps each stray byte as a
surrogate, as CPython decodes it, so `b"a\xffb"` becomes `'a\udcffb'`.
`sys.argv[0]` is the path of the executable, where CPython gives the
path of the script. The names of `sys` and `math` cannot be assigned:
`sys.argv = ["x"]` is E0402, so change the list in place, with
`sys.argv.clear()` and `sys.argv.extend(["x"])`.

A `SystemExit` that nothing catches ends the program without a
traceback. Its exit status is 0 for no code or `None`, the low byte of
an `int`, and 1 for any other value, which is written to standard
error first. `except Exception` does not catch it, since `SystemExit`
derives from `BaseException`, as in CPython.

`print(..., file=sys.stderr)` writes to standard error, and
`flush=True` writes out what standard output holds. `input(prompt)`
writes `str(prompt)`, then reads a line of standard input without its
`\n`, and raises `EOFError` at the end of the input:

<!-- typy-expect: accepted -->
```python
import sys

print("arguments:", sys.argv[1:])
print("a warning", file=sys.stderr)
try:
    name = input("name: ")
except EOFError:
    name = "nobody"
sys.stdout.write("hello, " + name + "\n")
if name == "nobody":
    sys.exit("no name given")
```

`sys.version_info`, `sys.platform`, `sys.maxsize`, `sys.stdin` and the
other constants listed in LIB-13 are planned for 0.4 (E0201). The rest
of CPython's `sys`, such as `sys.modules` or `sys.getrefcount`, depends
on the interpreter and is E0202. The switches of lazy imports,
`sys.set_lazy_imports` and its companions, are E0202 as well (MOD-09):
make an import lazy in its own module, as
[Modules](modules.md) describes.

## Text files

`open(file, mode)` opens a text file in UTF-8, as CPython 3.15 does by
default (LIB-04, ADR-0019). `mode` is a string literal: `"r"` reads an
existing file, `"w"` empties or creates one, `"a"` writes at the end and
`"x"` creates a new file; each may add `"t"`. A file reads and writes as
CPython's does:

- `read(size)` gives `size` characters, or the rest of the file without
  `size`; `readline()` gives the next line with its `\n`, and an empty
  string at the end; `readlines()` gives the list of the lines left.
- Reading turns `\r\n` and a lone `\r` into `\n`, as CPython's
  universal newlines do.
- `write(text)` returns the number of characters, and `writelines`
  writes each `str` of a list, a tuple or any iterable.
- Iterating over a file gives its lines, so a file goes where an
  `Iterable[str]` is expected, and `next(file)` gives the next line.
- `with open(...) as file:` closes the file on every way out of the
  block. A file whose last reference goes is closed, and the files still
  open when the program ends are closed then.

<!-- typy-expect: accepted -->
```python
with open("notes.txt", "w") as notes:
    notes.write("first line\n")
    print("second line", file=notes)

with open("notes.txt") as notes:
    for number, line in enumerate(notes, 1):
        print(number, line.rstrip())

try:
    open("missing.txt")
except FileNotFoundError as error:
    print(error)
```

Opening, reading and writing raise CPython's exceptions with CPython's
text: `FileNotFoundError: [Errno 2] No such file or directory:
'missing.txt'`, `IsADirectoryError`, `FileExistsError` for `"x"`, a
`UnicodeDecodeError` for bytes that are not UTF-8, `ValueError: I/O
operation on closed file.`, and `io.UnsupportedOperation: not readable`
for reading a file opened for writing.

Writes wait in a buffer until it fills, until `flush()` or `close()`, or
until the program ends. Another file object that reads the file before
then sees only what was written out, and CPython's buffer sizes differ
between versions, so flush or close a file before reading it again.

A mode CPython refuses for every file, such as `"rw"`, is E0404 with
CPython's message. TyPy 0.4 adds what 0.3 refuses with E0201 (LIB-14):

- binary modes such as `"rb"`, modes with `+`, and a mode that is not a
  literal;
- encodings other than UTF-8, and the arguments `buffering`, `errors`,
  `newline`, `closefd` and `opener` other than their defaults;
- `seek`, `tell`, `truncate`, `fileno`, `isatty`, `readable`, `writable`
  and `seekable`, and the attributes `name`, `mode` and `closed`.

`close()` of `sys.stdout` or `sys.stderr` writes it out and leaves it
open in 0.3; closing them comes with the rest of LIB-14.

<!-- typy-expect: rejected E0201 at 1:22 -->
```python
data = open("x.bin", "rb")
```
