# Compound statements

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `if`, `elif` and `else` | CST-01 |
| `while` and `for`, each with an optional `else` | CST-02 |
| `for` over `range`, `str`, `bytes`, `list`, `tuple`, `dict`, `set`, the views of a dict, and `enumerate`, `zip` and `reversed` of them | CST-03 |
| `try` with `except` clauses, `else` and `finally`, described in [Exceptions](exceptions.md) | CST-04 |
| `for` over an instance of a class with `__iter__`, whose result has `__next__` | CST-08 |
| `iter` and `next`, with or without a default, and `for` over an iterator | CST-09 |
| `with`, with one or more context managers and `as` targets | CST-05, CST-10 |

`match` comes in 0.5 (CST-06).

## Example

<!-- typy-expect: accepted -->
```python
prices = {"tea": 3, "cake": 5}
for name, price in prices.items():
    if price > 4:
        print(name, "is dear")
    elif price > 2:
        print(name, "is fair")
    else:
        print(name, "is cheap")
for index, letter in enumerate(reversed("abc"), start=1):
    print(index, letter)
count = 0
while count < 3:
    count += 1
else:
    print("counted to", count)
```

A loop's variable has one type, so a loop over a tuple of mixed items is
refused:

<!-- typy-expect: rejected E0411 at 1:13 -->
```python
for item in (1, "a"):
    print(item)
```

## Context managers

`with manager as target:` calls the manager's `__enter__`, binds what it
returns to the target, and calls `__exit__` on every way out of the body:
at its end, on `return`, `break` and `continue`, and when an exception
leaves it. `__exit__` takes the class of the exception, the exception and
its traceback, or three `None` values; its parameters may be left without
annotations (CST-10). When it returns a true value, the exception is
suppressed and the code after the `with` runs. Several managers in one
`with` nest, the first outermost.

<!-- typy-expect: accepted -->
```python
class Section:
    def __init__(self, title: str) -> None:
        self.title = title

    def __enter__(self) -> str:
        print("begin", self.title)
        return self.title.upper()

    def __exit__(self, kind, error, trace) -> bool:
        print("end", self.title, error)
        return kind is ValueError


with Section("intro") as heading:
    print(heading)
    raise ValueError("handled by __exit__")
print("after")
```

TyPy keeps no frames in a traceback object, so `trace` is one object
written as `<traceback object at 0x...>`, or `None`.

## Iterators

A class is iterable when it defines `__iter__`, which takes only `self`
and returns an instance of a class that defines `__next__`, often the
instance itself. `for` calls `__iter__` once, then `__next__` until it
raises `StopIteration`, which ends the loop; any other exception goes on.

<!-- typy-expect: accepted -->
```python
class Countdown:
    def __init__(self, start: int) -> None:
        self.current = start

    def __iter__(self) -> "Countdown":
        return self

    def __next__(self) -> int:
        if self.current <= 0:
            raise StopIteration
        self.current -= 1
        return self.current + 1


for number in Countdown(3):
    print(number)
```

`iter(value)` gives an iterator over a list, tuple, `str`, `bytes`, set,
dict or view of a dict, as CPython's: a list iterator sees items appended
while it runs, and a set or dict that changes size raises `RuntimeError`.
`iter` of an iterator gives the iterator itself, and of an instance calls
its `__iter__`. `next(iterator)` gives the next item, or raises
`StopIteration` at the end; `next(iterator, default)` gives the default
there, which has the item type or is `None`, making the result optional.

<!-- typy-expect: accepted -->
```python
words = iter(["alpha", "beta"])
print(next(words), next(words), next(words, None))
```

A function takes and returns iterators with the annotations `Iterator[T]`
and `Iterable[T]`, described in [Types](types.md) (TYP-15).
