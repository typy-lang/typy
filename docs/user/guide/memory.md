# Memory

TyPy frees memory by reference counting: an object is freed when its last
reference goes, with no garbage collector and no pauses.

## What TyPy 0.2 accepts

| Item | Rule |
| --- | --- |
| An object is freed when its last reference goes | MEM-01 |
| `float`, `bool` and inline `int` values are held without a heap allocation, in variables, attributes and containers, so a `list[float]` stores 8 bytes per item | MEM-02 |
| Passing a variable or a constant as an argument counts nothing, and `return name` moves the value to the caller | MEM-04 |
| An object header takes 8 bytes, and a million small `int` items fit in 8.5 MB | NFR-12, NFR-13 |

Moving a variable at its last use elsewhere comes in 0.3 (MEM-09),
allocation on the stack in 0.4 (MEM-03), and the collection of reference
cycles in 1.0 (MEM-06). Until then, objects that refer to each other in a
cycle stay allocated until the program ends.

## Example

Two names bound to one list share it, as in Python, and the list is freed
when the last name lets go of it:

<!-- typy-expect: accepted -->
```python
def build(count: int) -> list[float]:
    values: list[float] = []
    for index in range(count):
        values.append(index / 2)
    return values


data = build(5)
alias = data
alias.append(9.5)
print(data, len(alias))
data = [0.0]
print(alias[-1], data)
```

Memory management needs nothing from the program, so this area has no
rule of its own a program can break. A list stores each item by its type,
so a `list[float]` holds no `int`:

<!-- typy-expect: rejected E0409 at 2:11 -->
```python
values: list[float] = []
values = [1]
```
