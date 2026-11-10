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
| The set methods that combine a set with other collections | DAT-15 |

The comparisons of dict views and the views' set operations are planned
for 0.3 (DAT-16).

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
