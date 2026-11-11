# Simple statements

## What TyPy accepts

| Item | Rule |
| --- | --- |
| Assignment, annotated assignment and augmented assignment | SMT-01 |
| Unpacking into several targets from a tuple or a list, nested as in Python | SMT-02 |
| `pass`, `return`, `break` and `continue` | SMT-03 |
| `raise` and `assert`, described in [Exceptions](exceptions.md) | SMT-04, SMT-06 |
| `del` of list items, list slices and dict keys | SMT-05 |

`del` removes what each target names, in order: `del xs[i]` with
`IndexError` outside the list, `del xs[a:b:c]` what the slice selects, and
`del d[k]` with `KeyError` for a missing key. A variable or an attribute
cannot be deleted, E0421, since TyPy keeps each variable and attribute
bound once assigned.

<!-- typy-expect: accepted -->
```python
queue = [3, 1, 4, 1, 5]
del queue[0], queue[::2]
ages = {"ann": 31, "bob": 27}
del ages["bob"]
print(queue, ages)
```

`import` and `global` come in 0.3 (SMT-07, SMT-08), and
`nonlocal` in 1.0 (SMT-09).

## Example

<!-- typy-expect: accepted -->
```python
low, high = 1, 10
low, high = high, low
point: tuple[int, int] = (3, 4)
x, y = point
total = 0
total += x * y
items = [1, 2, 3]
items[0] += 10
print(low, high, x, y, total, items)
```

The number of targets must match the number of items:

<!-- typy-expect: rejected E0308 at 1:8 -->
```python
a, b = (1, 2, 3)
```
