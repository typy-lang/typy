# Simple statements

## What TyPy accepts

| Item | Rule |
| --- | --- |
| Assignment, annotated assignment and augmented assignment | SMT-01 |
| Unpacking into several targets from a tuple or a list, nested as in Python | SMT-02 |
| `pass`, `return`, `break` and `continue` | SMT-03 |
| `raise` and `assert`, described in [Exceptions](exceptions.md) | SMT-04, SMT-06 |
| `del` of list items, list slices and dict keys | SMT-05 |
| `global` in a function, for the module's variables | SMT-08 |

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

A function reads the variables of its module, and assigns them after
`global`. A read before the module binds the variable raises
`NameError`, as in CPython:

<!-- typy-expect: accepted -->
```python
count = 0


def bump() -> int:
    global count
    count += 1
    return count


bump()
print(bump(), count)
```

The name must be bound or annotated at module level, such as
`count: int`, which gives the variable its type, E0423 otherwise. CPython's
rules for `global` hold: it comes before any use of the name in the
function, and names no parameter, E0103.

A test such as `if label is not None:` does not narrow a module variable
that a function declares `global`, since any call may assign it. Copy
the value to a local variable and test that.

`import` is described in [Modules](modules.md), and `nonlocal` comes in
1.0 (SMT-09).

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
