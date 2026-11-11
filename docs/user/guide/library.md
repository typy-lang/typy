# Built-in functions and library

## What TyPy 0.2 accepts

| Item | Rule |
| --- | --- |
| `print` with `sep` and `end`, `len`, `range`, `abs`, `min`, `max`, `int`, `float`, `str`, `bool`, `repr`, `ord` and `chr` | LIB-01 |
| `list`, `dict`, `set`, `tuple`, `sorted`, `reversed`, `enumerate`, `zip`, `sum`, `any`, `all`, `round` and `type` | LIB-02, CLS-12 |
| The methods of `str`, `bytes`, `list`, `tuple`, `dict` and `set`, with CPython's results | LIB-03, DAT-15 |

`str` has `find`, `index`, `count`, `split`, `join`, `strip`, `replace`,
`startswith`, `upper`, `lower`, `title`, `center`, `zfill`, `encode`, the
`is` tests and the rest of the methods that read or rebuild text; `bytes`
has the same with `decode` and `hex`. Lists have `append`, `pop`,
`insert`, `extend`, `remove`, `index`, `count`, `clear`, `copy`,
`reverse` and `sort`; dicts `get`, `setdefault`, `pop`, `popitem`, `keys`,
`values`, `items`, `update`, `copy` and `clear`; sets `add`, `remove`,
`discard`, `pop` and the methods that combine sets.

`sys`, `input` and `open` come in 0.3 (LIB-04), the `math` module in 0.3
(LIB-05), and `%` formatting of strings in 0.3 (LIB-10). `iter` and `next`
are described in [Compound statements](compound-statements.md#iterators).

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

`%` formatting is refused until 0.3; an f-string formats now:

<!-- typy-expect: rejected E0201 at 1:7 -->
```python
print("%d items" % 3)
```
