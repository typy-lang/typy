# Built-in functions and library

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `print` with `sep` and `end`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `bool`, `repr`, `ord` and `chr` | LIB-01 |
| `list`, `dict`, `set`, `tuple`, `sorted`, `reversed`, `enumerate`, `zip`, `sum`, `any`, `all`, `round` and `type` | LIB-02, CLS-12 |
| The methods of `str`, `bytes`, `list`, `tuple`, `dict` and `set`, with CPython's results | LIB-03, DAT-15 |
| `format % values`, printf-style formatting of a string literal | LIB-10 |

`str` has `find`, `index`, `count`, `split`, `join`, `strip`, `replace`,
`startswith`, `upper`, `lower`, `title`, `center`, `zfill`, `encode`, the
`is` tests and the rest of the methods that read or rebuild text; `bytes`
has the same with `decode` and `hex`. Lists have `append`, `pop`,
`insert`, `extend`, `remove`, `index`, `count`, `clear`, `copy`,
`reverse` and `sort`; dicts `get`, `setdefault`, `pop`, `popitem`, `keys`,
`values`, `items`, `update`, `copy` and `clear`; sets `add`, `remove`,
`discard`, `pop` and the methods that combine sets.

`sys`, `input` and `open` come in 0.3 (LIB-04), and the `math` module in
0.3 (LIB-05). `iter` and `next` are described in
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
