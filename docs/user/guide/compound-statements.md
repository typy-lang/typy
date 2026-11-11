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

`with` comes in 0.3 (CST-05), and `match` in 0.5 (CST-06).

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

A variable holds an iterator by inference until the annotation
`Iterator[T]` arrives with imports in 0.3 (TYP-15).
