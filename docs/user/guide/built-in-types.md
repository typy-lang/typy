# Built-in types

TyPy's built-in types behave as CPython's do, with the same results and
the same exceptions.

## What TyPy 0.2 accepts

| Item | Rule |
| --- | --- |
| `int` without a size limit; values that fit in 63 bits are held inline | DAT-01 |
| `float`, an IEEE 754 double, written as CPython writes it | DAT-02 |
| `bool`; arithmetic on `bool` is refused | DAT-03 |
| `None` | DAT-04 |
| `str`, a sequence of Unicode code points | DAT-05 |
| `bytes`, a sequence of integers from 0 to 255 | DAT-06 |
| `list[T]`, shared by every name bound to it | DAT-07 |
| `tuple[A, B]` with a fixed number of items, and `tuple[T, ...]` | DAT-08, DAT-14 |
| `dict[K, V]`, in insertion order | DAT-09 |
| `set[T]`, in CPython's order for `int`, `str` and `bytes` items | DAT-10 |
| Negative indexes and `IndexError` outside a sequence | DAT-11 |
| Slices with start, stop and step | DAT-12 |
| Assigning a list to a slice of a list, `xs[a:b] = items` | DAT-13 |
| The set methods that combine a set with other collections | DAT-15 |
| `range` as a value: `len`, indexing, slicing, `in`, `==` and its text | DAT-17 |
| `dict.keys()` and `dict.items()` as sets: `\|`, `&`, `-`, `^` and comparisons | DAT-16 |
| `reversed` of a dict, `dict(name=value)` and `dict.fromkeys` | DAT-16 |

A slice without a step, or with a step of 1, takes any number of items,
and the list grows or shrinks to fit: `xs[:0] = items` inserts at the
front and `xs[:] = []` empties the list. A slice with any other step takes
exactly as many items as it selects, or raises `ValueError`, as CPython
does.

A `range` is a value you can store, pass and annotate. Its bounds may be
of any size, as in CPython, but `len()` of a range longer than
`sys.maxsize` raises `OverflowError`, and so does slicing it:

<!-- typy-expect: accepted -->
```python
def evens(limit: int) -> range:
    return range(0, limit, 2)


numbers = evens(10)
print(numbers, len(numbers), numbers[-1], numbers[1:3], 4 in numbers)
print(list(reversed(numbers)), numbers == range(0, 9, 2))
```

The views `d.keys()` and `d.items()` act as sets of their items. `|`,
`&`, `-` and `^` with a set, another view, or a list of the same items
give a new `set`, in CPython's order. `==`, `<` and the other comparisons
with a set or another view compare the items:

<!-- typy-expect: accepted -->
```python
stock = {1: 10, 2: 0, 3: 7}
wanted = {2, 3, 4}
print(stock.keys() & wanted, stock.keys() - wanted)
print(stock.items() & {(3, 7), (4, 1)}, stock.keys() <= {1, 2, 3, 4})
```

A set needs items with a hash, so `d.items()` of a dict whose values are
lists compares with another view but makes no set. `d.values()` is no
set: it compares only with itself, by identity, as in CPython.
`reversed(d)`, and `reversed` of a view, give the entries from the last
to the first. `dict(name=value)` makes a dict of `str` keys, after the
pairs of a source when one is given, and `dict.fromkeys(keys, value)`
maps every key to the one value, or to `None`:

<!-- typy-expect: accepted -->
```python
defaults = {"color": "red", "size": "m"}
print(dict(defaults, size="l"), list(reversed(defaults)))
print(dict.fromkeys(["ada", "alan"], 0), dict.fromkeys("ab"))
```

## Example

<!-- typy-expect: accepted -->
```python
huge = 2 ** 100
values = [3, 1, 2]
shared = values
shared.append(4)
pair = ("x", 1.5)
first, second = pair
seen = {1, 2, 3} | {3, 4}
ages = {"ada": 36, "alan": 41}
print(huge, values, values[-1], values[1:3], first, second)
print(sorted(seen), list(ages.items()), b"ab" + b"c", "naive"[::-1])
```

Every item of a list, set or dict display has one type:

<!-- typy-expect: rejected E0409 at 1:13 -->
```python
mixed = [1, "two"]
```
