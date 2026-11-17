# Memory

TyPy frees memory by reference counting: an object is freed when its last
reference goes, with no garbage collector and no pauses.

## What TyPy accepts

| Item | Rule |
| --- | --- |
| An object is freed when its last reference goes | MEM-01 |
| `float`, `bool` and inline `int` values are held without a heap allocation, in variables, attributes and containers, so a `list[float]` stores 8 bytes per item | MEM-02 |
| Passing a variable or a constant as an argument counts nothing, and `return name` moves the value to the caller | MEM-04 |
| The last use of a variable moves its value into an assignment or a collection, and a new value, such as a call's result, goes into a collection without being counted | MEM-09 |
| An instance of a class of the program that never outlives the function that makes it is made in the function's frame, with no allocator call | MEM-03 |
| A project names the C functions its program allocates with in `typy.toml` | MEM-08 |
| An object header takes 8 bytes, and a million small `int` items fit in 8.5 MB | NFR-12, NFR-13 |

A move changes no result: the value goes where CPython would put it, and
an object is still freed when its last reference goes. In this loop,
`row` and `name` move into the lists, so nothing is counted:

<!-- typy-expect: accepted -->
```python
def table(count: int) -> list[list[str]]:
    rows: list[list[str]] = []
    for index in range(count):
        name = "row " + str(index)
        row = [name]
        rows.append(row)
    return rows


print(table(3))
```

An instance stays in its function's frame when the function reads it,
assigns its attributes, calls its methods, or passes it to functions that do
no more. Returning it, storing it in a container, in an attribute or in a
module variable, or passing it to a built-in function other than
`isinstance` and `type`, such as `print` or `str`, makes it on the heap.
The instance behaves the same either way. In this loop, each `Vector` is
made in the frame of `total`:

<!-- typy-expect: accepted -->
```python
class Vector:
    def __init__(self, x: float, y: float) -> None:
        self.x = x
        self.y = y

    def dot(self, other: "Vector") -> float:
        return self.x * other.x + self.y * other.y


def total(count: int) -> float:
    result = 0.0
    for index in range(count):
        a = Vector(float(index), 1.0)
        b = Vector(2.0, float(index))
        result += a.dot(b)
    return result


print(total(4))
```

`typy build --emit=tyir` ends the line of each such instance with
`in frame`. Lists, dicts, sets, strings and exceptions come from the heap
until 1.0 (MEM-10), and the collection of reference cycles comes in 1.0
(MEM-06). Until then, objects that refer to each other in a cycle stay
allocated until the program ends.

## Choosing the allocator

A program allocates its memory with the C library's `malloc`, `realloc`
and `free`. A project can name three other C functions in `typy.toml`, and
the library that defines them, with no change to its source (MEM-08). To
use [mimalloc](https://github.com/microsoft/mimalloc):

```toml
[allocator]
allocate = "mi_malloc"
reallocate = "mi_realloc"
free = "mi_free"
library = "mimalloc"
```

| Key | Value |
| --- | --- |
| `allocate` | A function `void *allocate(size_t size)`, as `malloc` |
| `reallocate` | A function `void *reallocate(void *pointer, size_t size)`, as `realloc` |
| `free` | A function `void free(void *pointer)`, as `free` |
| `library` | The library the linker finds as it finds `-lmimalloc`, in its usual directories or in `LIBRARY_PATH`; leave it out when a library the program links anyway defines the functions |

TyPy never asks for zero bytes, and needs memory aligned as `malloc`
aligns it. When `allocate` or `reallocate` returns null, the program
prints `MemoryError` and stops. The three keys come together, each a C
function's name that does not start with `typy_`, `__typy` or `_TY`, which
belong to TyPy; a table that leaves one out, or names one function twice,
is E0107.

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
