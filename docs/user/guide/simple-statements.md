# Simple statements

## What TyPy 0.2 accepts

| Item | Rule |
| --- | --- |
| Assignment, annotated assignment and augmented assignment | SMT-01 |
| Unpacking into several targets from a tuple or a list, nested as in Python | SMT-02 |
| `pass`, `return`, `break` and `continue` | SMT-03 |

`assert`, `del`, `raise`, `import` and `global` come in 0.3 (SMT-04 to
SMT-08), and `nonlocal` in 1.0 (SMT-09).

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
