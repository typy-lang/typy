# Simple statements

## What TyPy accepts

| Item | Rule |
| --- | --- |
| Assignment, annotated assignment and augmented assignment | SMT-01 |
| Unpacking into several targets from a tuple or any iterable, nested as in Python, with one starred target that takes the rest as a list | SMT-02, EXP-17 |
| `pass`, `return`, `break` and `continue` | SMT-03 |
| `raise` and `assert`, described in [Exceptions](../library/exceptions.md) | SMT-04, SMT-06 |
| `del` of list items, list slices and dict keys | SMT-05 |
| `global` in a function, for the module's variables | SMT-08 |
| `nonlocal` in a nested function, for the variables of the functions around it | SMT-09 |
| `type` at module level, described in [Types](types.md#type-aliases) | SMT-10 |

## Unpacking

Targets separated by commas take the values of the value assigned, in
order. A tuple of a fixed number of items gives each target its own type,
and the number of targets must match, E0308. Any other iterable, such as
a list, a `str`, a range or a generator, gives every target its item type,
and the number is checked as the program runs, with CPython 3.15's
`ValueError`. One target may be starred, `*rest`, to take a list of the
values the others leave:

<!-- typy-expect: accepted -->
```python
first, *rest = [1, 2, 3]
head, *middle, tail = "abcd"
for key, *values in [["k", "v1", "v2"], ["j"]]:
    print(key, values)
print(first, rest, head, middle, tail)
```

A starred target over a tuple of a fixed number of items takes those
between the others, which must share one type. As in CPython, a target
without a star reads one value past the targets of an iterator to find
too many, so an endless generator raises `ValueError` and does not hang.

## Deleting

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

## Variables of the module and of enclosing functions

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

A nested function that declares a name `nonlocal` reads and assigns the
variable of that name of the nearest function around it that binds it, an
annotation alone such as `count: int` included. The variable lives in a
cell the functions share, so each sees every assignment, as in CPython:

<!-- typy-expect: accepted -->
```python
from typing import Callable


def make_counter() -> Callable[[], int]:
    count = 0

    def increment() -> int:
        nonlocal count
        count += 1
        return count
    return increment


counter = make_counter()
counter()
print(counter(), counter())
```

An assignment keeps the variable's type, E0402 otherwise. CPython's rules
for `nonlocal` hold, E0103: it is not allowed at module level, a function
around it binds the name, and it comes before any use of the name in the
function and names no parameter. A nested function that reads the
variable before the function around it assigns one raises `NameError`.
The function around it reads the variable only after it assigns it
itself, E0306 otherwise (TYP-12), and a nested function's name cannot be
assigned through `nonlocal`, E0202, since only its `def` binds it.

A test such as `if value is not None:` does not narrow a variable that a
nested function assigns through `nonlocal`, since any call may assign it.
Copy the value to a local variable and test that, as for `global`.

An `except ... as name` clause deletes `name` when it ends, so `name`
cannot be a variable declared `global` or `nonlocal`, or one that a
nested function shares, E0202: TyPy gives the clause a variable of its
own, which the other functions would not see.

`import` is described in [Modules](modules.md).

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
