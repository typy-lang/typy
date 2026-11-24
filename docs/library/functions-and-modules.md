# Built-in functions and modules

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `print` with `sep` and `end`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `bool`, `repr`, `ord` and `chr` | LIB-01 |
| `list`, `dict`, `set`, `tuple`, `sorted`, `reversed`, `enumerate`, `zip`, `sum`, `any`, `all`, `round` and `type` | LIB-02, CLS-12 |
| The `key` argument of `sorted`, `list.sort`, `min` and `max`, a function value of one item | FUN-05 |
| The methods of `str`, `bytes`, `list`, `tuple`, `dict` and `set`, with CPython's results | LIB-03, DAT-15 |
| `format % values`, printf-style formatting of a string literal | LIB-10 |
| The `math` module: its constants and its functions of numbers | LIB-05 |
| `sys.argv`, `sys.exit`, `sys.stdout` and `sys.stderr`, `print` with `file` and `flush`, and `input` | LIB-04 |
| `open` for text files in UTF-8, with `read`, `readline`, `readlines`, `write`, `writelines`, `flush`, `close`, iteration and `with` | LIB-04 |
| `callable(x)`: true for a function value, a built-in function, method or type, and a class | LIB-23 |
| `map` and `filter`, lazy iterators that go on after the function raises | LIB-23 |
| `divmod`, `pow` with or without a modulus, `bin`, `hex`, `oct` and `format`; see [Numbers and formatting](#numbers-and-formatting) | LIB-28 |
| `hash`, the hash a set or a dict keeps a value by; see [Hashing](#hashing) | LIB-34 |

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
[Compound statements](../reference/compound-statements.md#iterators).
`callable(x)` is decided when the program is compiled for every type but
an optional or a union, which it tests as `isinstance` does, and it still
evaluates `x`.

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

## Ordering by a key

The `key` of `sorted`, `list.sort`, `min` and `max` is a function value
that takes one item, a function or a method of the program, such as a
bound method. Its results are the keys, which must order with `<`, and
the items are ordered or compared by their keys alone, as in CPython:

<!-- typy-expect: accepted -->
```python
def size(word: str) -> int:
    return len(word)


def score(pair: tuple[str, int]) -> int:
    return pair[1]


words = ["pear", "fig", "banana", "kiwi"]
pairs = [("b", 2), ("a", 2), ("c", 1)]
print(sorted(words, key=size), sorted(words, key=size, reverse=True))
print(min(words, key=size), max(pairs, key=score), max("b", "aa", key=size))
pairs.sort(key=score)
print(pairs, min(words[:0], key=size, default="none"))
```

`sorted` and `list.sort` call the key once for each item, in order, then
sort stably, so items of equal keys keep their order, also with
`reverse=True`. `min` and `max` compare each key with the best so far as
soon as it is computed, and give the first item of the smallest or the
largest key. `key=None` is no key. `list.sort` orders a copy of the items
and writes them back, so a key that raises leaves the list as it was, as
in CPython; CPython leaves undefined what a key sees of the list while it
is sorted, and in TyPy it sees the items unsorted. A built-in function is
a key too, such as `key=len` or `key=abs`, called with each item.

## Mapping and filtering

`map(function, iterable, ...)` gives an iterator of the function's
results for the items of the iterables, taken together and stopping at
the shortest; `filter(function, iterable)` gives the items for which the
function's result is true, and `filter(None, iterable)` the true items.
The function is a function or method of the program, a `lambda`, or a
built-in such as `int` or `str.upper`, and takes the items as they are.
Each step calls the function once, and when it raises, the next step goes
on with the next items, as in CPython (LIB-23, ADR-0089):

<!-- typy-expect: accepted -->
```python
words = ["3", "x", "12"]
numbers = map(int, words)
print(next(numbers))
try:
    next(numbers)
except ValueError as error:
    print("skipped:", error)
print(list(numbers), list(filter(str.isdigit, words)))
print(list(map(lambda a, b: a * b, [1, 2, 3], [4, 5])))
```

A `map` or `filter` object is an iterator: a `for` loop, `list`, `next`
and a parameter of type `Iterator[T]` take it, and it is written as
`<map object at 0x...>`. With a Python floor of 3.14 or later, `map`
takes `strict=True`, which raises CPython's `ValueError` when the
iterables differ in length; below 3.14 it is refused, as CPython 3.12
and 3.13 raise `TypeError` for it. `map` and `filter` themselves as
values, such as `f = map`, and `*` in a call of `map` are planned for 1.0
(LIB-25).

## Numbers and formatting

`divmod(a, b)` gives `(a // b, a % b)`, of `int`s for two integers and of
`float`s otherwise. `pow(a, b)` is `a ** b`, so its exponent follows the
rule for `**`: an `int` exponent of an `int` is a non-negative integer
literal (EXP-04). `pow(a, b, m)` of three integers reduces the power
modulo `m` at every step, gives a result with the sign of `m`, and takes a
negative exponent when `a` has an inverse modulo `m`:

<!-- typy-expect: accepted -->
```python
print(divmod(17, 5), divmod(-7.5, 2))
print(pow(2, 10), pow(2, 10**20, 97), pow(3, -1, 7))
print(bin(10), hex(255), oct(-8))
print(format(1234567.891, ",.2f"), format("id", ">6"), format(42))
```

`bin`, `hex` and `oct` write an `int` with the prefix `0b`, `0x` or `0o`,
and `format(value, spec)` formats a value as the f-string `f"{value:spec}"`
does. A modulus of zero, or a negative exponent whose base has no inverse,
raises `ValueError`. `bytearray` is described in
[Built-in types](built-in-types.md#bytearrays).

## Hashing

`hash(x)` gives the hash a set or a dict keeps `x` by (LIB-34). For an
`int`, a `float`, a `bool`, `None` and a tuple of them it is CPython's;
for an instance it is what the class's `__hash__` returns, so `__hash__`
can hash a tuple of the attributes `__eq__` compares. `str` and `bytes`
hash with CPython's function under a fixed key, where CPython draws a key
at random for each run, so their hashes are not promised (STD-03):

<!-- typy-expect: accepted -->
```python
class Point:
    def __init__(self, x: int, y: int) -> None:
        self.x = x
        self.y = y

    def __eq__(self, other: "Point") -> bool:
        return (self.x, self.y) == (other.x, other.y)

    def __hash__(self) -> int:
        return hash((self.x, self.y))


print(hash(Point(1, 2)) == hash((1, 2)), len({Point(1, 2), Point(1, 2)}))
```

A value of a type that has no hash, such as a list, is E0413. In a copy
of a generic function or class whose type argument has no hash, the call
raises `TypeError` where it runs, as CPython raises it.

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

TyPy reads a literal format when it checks the program, so one that does
not fit its values is E0427, with the message CPython's `TypeError` or
`ValueError` would give, in CPython 3.15's words, such as
`not enough arguments for format string (got 1)`:

<!-- typy-expect: rejected E0427 at 1:25 -->
```python
print("%d of %d done" % (3,))
```

A format that is not a literal, such as one read from a variable, and a
`tuple[T, ...]` of values, are read when the program runs (LIB-11): the
runtime gives the same text, and raises the `TypeError`, `ValueError` or
`KeyError` CPython would raise:

<!-- typy-expect: accepted -->
```python
template = "%s scored %d"
results: tuple[int, ...] = (3, 4)
print(template % ("Ada", 7), "%d-%d" % results)
try:
    print(template % "only one")
except TypeError:
    print("not enough values")
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

From the Python floor 3.15, the submodule `math.integer` gives the
functions of `math` that take integers, `comb`, `factorial`, `gcd`,
`isqrt`, `lcm` and `perm` (LIB-26): `import math.integer as mi` with
`mi.gcd(12, 18)`, `from math.integer import isqrt`, or
`math.integer.lcm(4, 6)` after `import math.integer`. Below that floor,
`math` is no package and `math.integer` is E0206, as in CPython 3.14.
A function of `math` or `sys` is a value where a `Callable` is expected,
such as `sorted(values, key=math.fabs)` or `root: Callable[[float], float]
= math.sqrt`, written `<built-in function sqrt>`; with no `Callable`
expected it is E0203 (FUN-08).

`fsum`, `dist`, `prod` and `sumprod` take iterables of `int`, `bool` or
`float` items, and `gamma` and `lgamma` a real number, each with
CPython's result (LIB-12). As in CPython, `prod` of `float`s with the
default start and `sumprod` with a `float` give the `int` start, or 0,
when there are no items, so their type is `int | float`; give `prod` a
`float` start for a `float`:

<!-- typy-expect: accepted -->
```python
import math

prices = [2.5, 4.0]
counts = [3, 2]
print(math.fsum([0.1] * 10), math.dist((0, 0), (3, 4)))
print(math.prod(range(1, 6)), math.prod(prices, start=1.0))
print(math.sumprod(prices, counts), math.gamma(0.5), math.lgamma(10.0))
```

Items of a union of number types, such as a `list[int | float]`, and a
`bool` start of `prod` give CPython's results too (LIB-15): `prod` keeps
the exact `int` product until its first `float` item, and `prod([],
start=True)` is `True`. `prod` of items that are not numbers, such as
strings, is planned for 1.0 (LIB-21).

## The bisect module

`import bisect` gives CPython's `bisect` module, written in typed Python
that CPython also runs (LIB-27): `bisect_left`, `bisect_right` and
`bisect` find where a value goes in a sorted sequence, and `insort_left`,
`insort_right` and `insort` put it there in a list, each with `lo` and
`hi` bounds and CPython's `ValueError` for a negative `lo`. The items
compare as their types do, so a list of tuples is sorted item by item
(GEN-18):

<!-- typy-expect: accepted -->
```python
import bisect

grades = [60, 70, 80, 90]
events = [(1, "start"), (5, "stop")]
bisect.insort(events, (3, "pause"))
print(bisect.bisect(grades, 75), bisect.bisect_left(grades, 80), events)
```

The `key` parameter, which makes the searched value of another type than
the items, needs `@overload` with type parameters and is planned for
0.13 (TYP-35).

## The heapq module

`import heapq` gives CPython's `heapq` module in typed Python (LIB-27):
`heappush`, `heappop`, `heappushpop`, `heapreplace` and `heapify` keep a
list as a heap, `merge` merges sorted iterables lazily, with `reverse`,
and `nlargest` and `nsmallest` pick items, ties broken as CPython breaks
them. An empty heap raises CPython's `IndexError: index out of range`:

<!-- typy-expect: accepted -->
```python
import heapq

tasks = [(2, "write"), (1, "plan"), (3, "test")]
heapq.heapify(tasks)
print(heapq.heappop(tasks), heapq.nlargest(2, [5, 1, 9, 4]))
print(list(heapq.merge([1, 4, 7], [2, 5, 8])))
```

`key` of `merge`, `nlargest` and `nsmallest` is planned for 0.13
(TYP-35), as is the `heapify_max` family Python 3.14 adds (LIB-32).

## The string module

`import string` gives CPython's `string` module in typed Python
(LIB-27): its constants, such as `ascii_letters`, `digits` and
`punctuation`, `capwords`, and `Template`, whose `substitute`,
`safe_substitute`, `is_valid` and `get_identifiers` read `$name`,
`${name}` and `$$` as CPython's default pattern does, with a class's own
`delimiter`. A placeholder of no form raises CPython's `ValueError` with
its line and column:

<!-- typy-expect: accepted -->
```python
from string import Template, capwords

print(capwords(" hello  world "), Template("$who: ${n} $$").substitute(who="a", n=1))
print(Template("$a $b").safe_substitute({"a": "x"}), Template("$a").get_identifiers())
```

`Formatter`, and a `Template` subclass's own `idpattern`,
`braceidpattern`, `flags` or `pattern`, which CPython compiles into a
regular expression, are planned for 0.13 (TYP-25).

## The textwrap module

`import textwrap` gives CPython's `textwrap` module in typed Python
(LIB-27): `TextWrapper` with each of its options, and `wrap`, `fill`,
`shorten`, `dedent` and `indent`. Text is split into words, whitespace
and the parts of hyphenated words as CPython 3.15 splits it with its
regular expression:

<!-- typy-expect: accepted -->
```python
import textwrap

text = "The well-known rule: wrap at word boundaries, never mid-word."
print(textwrap.wrap(text, 20))
print(textwrap.shorten(text, 30), textwrap.dedent("  a\n    b"))
print(textwrap.indent("one\ntwo", "> "), sep="")
```

The attributes `wordsep_re`, `wordsep_simple_re` and `sentence_end_re`
of `TextWrapper`, which hold regular expressions, are not given, so a
subclass changes how text is split by overriding `_split`.

## The base64 module

`import base64` gives CPython's `base64` module in typed Python
(LIB-27): `b64encode` and `b64decode`, with `altchars` and `validate`,
their `standard_` and `urlsafe_` forms, `b32encode`, `b32decode`,
`b32hexencode`, `b32hexdecode`, `b16encode`, `b16decode`, `a85encode`,
`a85decode`, `b85encode`, `b85decode`, `encodebytes` and `decodebytes`.
A decoder takes `bytes` or a `str` of ASCII characters. The results and
messages are those of CPython 3.15, which reads malformed input
otherwise than 3.12 did: `urlsafe_b64decode` needs no padding, and
`binascii.Error`, a `ValueError`, says what is wrong with the input:

<!-- typy-expect: accepted -->
```python
import base64
import binascii

token = base64.urlsafe_b64encode(b"\xfb\xff typed")
print(token, base64.urlsafe_b64decode(token), base64.b32encode(b"ok"))
try:
    base64.b64decode("YQ")
except binascii.Error as error:
    print(error)
```

The parameters Python 3.13 to 3.15 add, such as `padded`, `wrapcol` and
`ignorechars`, and `z85encode` and `z85decode` are planned for 0.13
(LIB-32). `encode` and `decode`, which read and write binary files, need
`typing.BinaryIO`, planned for 1.0 (LIB-19). CPython 3.15 also writes a
`FutureWarning` when URL-safe data, or data decoded with `altchars`,
holds `+` or `/`; TyPy decodes the same bytes and writes no warning.

## The binascii module

`import binascii` gives the functions of CPython's `binascii` module
that `base64` uses, in typed Python (LIB-27): `b2a_base64` and
`a2b_base64`, with `newline` and `strict_mode`, `hexlify` and `b2a_hex`,
with `sep` and `bytes_per_sep`, `unhexlify` and `a2b_hex`, `crc32`, and
the exceptions `Error` and `Incomplete`:

<!-- typy-expect: accepted -->
```python
import binascii

print(binascii.hexlify(b"\xb9\x01\xef", "-"), binascii.unhexlify("b901ef"))
print(binascii.crc32(b"hello"), binascii.b2a_base64(b"hi", newline=False))
```

`a2b_uu`, `b2a_uu`, `a2b_qp`, `b2a_qp` and `crc_hqx` are planned for 0.13
(LIB-31).

## The sys module

`import sys` gives these names of CPython's `sys` module (LIB-04,
LIB-13):

| Name | What it is |
| --- | --- |
| `sys.argv` | A `list[str]`: the path of the executable, then each argument |
| `sys.exit(code)` | Raises `SystemExit(code)`; `code` may be left out |
| `sys.stdout`, `sys.stderr` | Text files with `write(text)`, which returns the number of characters, and `flush()` |
| `sys.stdin` | A text file with `read`, `readline`, `readlines` and iteration, which `input()` reads too |
| `sys.maxsize`, `sys.maxunicode` | `2**63 - 1` and `1114111` |
| `sys.byteorder`, `sys.platform` | `"little"` on every target; `"linux"` in a program built for Linux and `"darwin"` in one built for macOS |
| `sys.getrecursionlimit()`, `sys.setrecursionlimit(n)` | Read and set the recursion limit, 1000 at first |
| `sys.version`, `sys.hexversion`, `sys.version_info` | CPython 3.15.0, whose behaviour TyPy follows: `'3.15.0 (TyPy 0.12.0)'`, `0x030F00F0` and `(3, 15, 0, 'final', 0)` |

An argument that is not valid UTF-8 keeps each stray byte as a
surrogate, as CPython decodes it, so `b"a\xffb"` becomes `'a\udcffb'`,
and printing it writes the byte back.
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
`flush=True` writes out what standard output holds. `print` also takes
its keywords from a dict of `str` keys, `print(*values, **options)`,
bound when the call runs, with CPython's errors for a keyword given
twice or unknown, a `sep` or an `end` that is no `str`, and a `file`
without `write` (EXP-22). `input(prompt)`
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

`sys.stdin` reads standard input as CPython 3.15 does: `\r\n` and `\r`
become `\n`, and a byte that is not part of a UTF-8 character becomes a
surrogate, as for an argument. `input()` and `sys.stdin` share what has
been read, so they can be mixed. Writing to `sys.stdin` raises
`io.UnsupportedOperation: not writable`.

`setrecursionlimit` checks its argument as CPython does: below 1 is
`ValueError`, and past a C `int` is `OverflowError`. Recursion itself
stops where the stack runs out, with `RecursionError` (FUN-02), so the
number is kept only for `getrecursionlimit`, and a limit below the
current depth, which CPython refuses with `RecursionError`, is kept.

A program built by TyPy runs without an interpreter, so it reports the
version of CPython whose results and messages it has, 3.15, whatever its
floor (ADR-0029). `sys.version_info` is compared, indexed and read by
field where the program uses it, through `sys`:

<!-- typy-expect: accepted -->
```python
import sys

if sys.version_info >= (3, 12):
    print("major version", sys.version_info.major, sys.version_info[0])
print(sys.version.startswith("3."), sys.hexversion >= 0x030C0000)
```

`sys.version_info` is also a value of its own, as in CPython: it may be
stored, put in a container, imported with `from sys import version_info`
and printed, as
`sys.version_info(major=3, minor=15, micro=0, releaselevel='final', serial=0)`,
and indexing, slicing, comparing, `len`, a loop and unpacking read it as
the tuple of its items (LIB-16).

<!-- typy-expect: accepted -->
```python
import sys
from sys import version_info

kept = [version_info]
major, minor, micro, level, serial = kept[0]
print(major, level, kept[0] >= (3, 12), kept[0] is sys.version_info)
```

A function parameter cannot name its type, so pass `version_info[:]`,
a plain tuple, where a function takes the items; `tuple(version_info)`
and `in` are refused, as for any tuple whose items differ in type.
The rest
of CPython's `sys`, such as `sys.modules` or `sys.getrefcount`, depends
on the interpreter and is E0202. The switches of lazy imports,
`sys.set_lazy_imports` and its companions, are E0202 as well (MOD-09):
make an import lazy in its own module, as
[Modules](../reference/modules.md) describes.

## Text files

`open(file, mode)` opens a text file in UTF-8, as CPython 3.15 does by
default (LIB-04, ADR-0019). `mode` is a string literal: `"r"` reads an
existing file, `"w"` empties or creates one, `"a"` writes at the end and
`"x"` creates a new file; each may add `"t"`, and `"+"` to both read and
write the file (LIB-14). A file reads and writes as CPython's does:

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

A file tells about itself as CPython's does (LIB-14): `name` is the path
it was opened with, or `<stdin>`, `<stdout>` and `<stderr>`; `mode` is the
mode as written; `closed`, `fileno()`, `isatty()`, `readable()`,
`writable()` and `seekable()` answer as their names say. Once a file is
closed, `name`, `mode` and `closed` still answer, and the methods raise
`ValueError: I/O operation on closed file`, except that `readable()` of a
file opened for writing, and `writable()` of one opened for reading,
answer `False`.

`tell()` gives the file's position, which `seek(position)` returns to,
and `truncate(size)` cuts or extends the file to `size` bytes, or to its
position without `size`, without moving it. A position counts bytes of
UTF-8, as CPython's does for a file read or written in UTF-8. `seek(0,
1)` stays where it is, `seek(0, 2)` moves to the end, and a move by
another amount from either is `io.UnsupportedOperation`. A file opened
with `"a"` starts at its end. Once a `for` loop has taken a line from a
file, `tell()` raises `OSError: telling position disabled by next() call`
until the loop reaches the end or `seek` moves the file, as in CPython.

<!-- typy-expect: accepted -->
```python
with open("notes.txt", "w") as notes:
    notes.write("caf\u00e9\n")
    print(notes.name, notes.mode, notes.tell())

with open("notes.txt") as notes:
    print(repr(notes.read(3)), notes.tell())
    notes.seek(0)
    print(repr(notes.readline()), notes.closed)
```

CPython leaves a `seek` to a position `tell` did not give undefined, and
reads the bits of one from 2^64 up as the state of its decoder. TyPy
moves to such a position by bytes, and refuses one from 2^63 up with
`OSError: [Errno 22] Invalid argument`, which CPython raises from 2^63
to 2^64.

A file opened with `"+"` writes out what it holds before it reads. A
write after a read starts where the reading stopped taking bytes from the
file, which is the end of the 8192 bytes CPython's text layer decodes at
a time, not the position `tell()` gives: call `seek(file.tell())` before
writing to write there.

Writes wait in a buffer until it fills, until `flush()` or `close()`, or
until the program ends. Another file object that reads the file before
then sees only what was written out, and CPython's buffer sizes differ
between versions, so flush or close a file before reading it again.

### Encodings, errors and newlines

`open` takes `buffering`, `encoding`, `errors` and `newline` as CPython's
does (LIB-14):

- `encoding` names `utf-8`, `ascii` or `latin-1`, in any spelling CPython
  finds for them, such as `"UTF8"` or `"Latin1"`; the file's `repr` shows
  it as written. `errors` names `strict`, `ignore`, `replace`,
  `backslashreplace`, `xmlcharrefreplace` or `surrogateescape`. Both are
  string literals or `None`, so TyPy checks them when it compiles; another
  codec or handler is E0404. Reading with `xmlcharrefreplace` raises
  CPython's `TypeError` at the first byte the codec cannot decode.
- `newline` is `None`, `""`, `"\n"`, `"\r"` or `"\r\n"`, or a variable
  of type `str | None`, which is checked when the file is opened. With
  `None`, reading ends a line at `\n`, `\r\n` or `\r` and gives each as
  `\n`; with `""` it ends lines there and gives them as they are; with
  another value a line ends only at that value. Writing turns each `\n`
  into `"\r"` or `"\r\n"` when `newline` is one of them.
- `buffering=1` writes the file out after each write of a `\n` or a
  `\r`; another positive size keeps TyPy's own buffer, and `0` raises
  `ValueError: can't have unbuffered text I/O` after opening the file, as
  CPython does.

<!-- typy-expect: accepted -->
```python
with open("notes.txt", "w", encoding="latin-1", newline="\r\n") as notes:
    notes.write("caf\u00e9\n")

with open("notes.txt", encoding="ascii", errors="replace") as notes:
    print(repr(notes.read()))
```

A mode CPython refuses for every file, such as `"rw"`, is E0404 with
CPython's message, and so is a `newline` literal it refuses. `opener`
takes a function as a value and is planned for 1.0 (LIB-18).

`open(fd)` with an `int` opens a file over a descriptor the program
already has, such as `sys.stdout.fileno()`. Its `name` is that `int`, so
the file has a type of its own, such as `TextIOWrapper of a file
descriptor`, and a variable holds files of one sort only (LIB-14).
`closefd=False` leaves the descriptor open when the file closes, and with
a path it is E0404, as CPython raises `ValueError` for it. CPython 3.13
and later warn when a `bool` is used as a descriptor, so `open(True)` is
E0404.

A file that cannot be closed when its last reference goes, or when the
program ends, such as one whose descriptor another file has closed, is
dropped without a message, where CPython reports the error as an
ignored exception.

A mode held in a variable, such as `open(path, mode)`, opens a text file
(ADR-0031). It is checked when the file is opened, with CPython's
`ValueError` for a mode CPython refuses. A binary mode held in a variable
raises `TypeError`, where CPython opens a binary file: write a binary
mode as a literal, such as `open(path, "rb")`.

### Binary files

A mode with `"b"` opens a binary file, of the class CPython makes for it
(LIB-14, ADR-0031): `BufferedReader` for `"rb"`, `BufferedWriter` for
`"wb"`, `"ab"` and `"xb"`, `BufferedRandom` for a mode with `"+"`, and
`FileIO` for any of them with `buffering=0`. A binary file reads and
writes `bytes`: `read`, `readline` and `readlines` give `bytes`, a `for`
loop gives its lines as `bytes`, and `write` takes `bytes` and returns
their number. `seek` moves by any amount from the start, the position
(`whence=1`), the end (`whence=2`), or Linux's next data or hole (`3` and
`4`). Each class raises CPython's errors with its own words, such as
`ValueError: read of closed file` for a `BufferedReader` and `ValueError:
I/O operation on closed file` for a `FileIO`. The `mode` of a binary
file is shown as CPython 3.15 shows it, such as `"wb+"` for `"w+b"`,
where 3.12 to 3.14 show `"rb+"`.

<!-- typy-expect: accepted -->
```python
with open("data.bin", "wb") as out:
    out.write(b"\x00\x01line\n")

with open("data.bin", "rb") as data:
    print(data.read(2), data.readline(), data.tell())
```

A binary mode takes no `encoding`, `errors` or `newline`, as CPython
raises `ValueError` for them, so each is E0404. `buffering=1` is E0404
too, since CPython warns with the program's own file and line. A
`buffering` that is not a literal decides the class only when the
program runs, and is planned for 1.0 with `typing.BinaryIO` (LIB-19).

`close()` of `sys.stdin`, `sys.stdout` or `sys.stderr` closes it as
CPython does: what it holds is written out, and then writing to it,
reading it, `input()` and `print()` to it raise `ValueError: I/O
operation on closed file.` The descriptor stays open, as CPython opens
the three with `closefd=False`. When `sys.stderr` is closed, `sys.exit`
with a message writes only a line ending, and an uncaught exception
writes `lost sys.stderr`; CPython writes a dump of the exception object,
with its address, before that line.

<!-- typy-expect: rejected E0201 at 1:29 -->
```python
data = open("x.txt", opener=len)
```
