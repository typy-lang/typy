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
| `match` with literal, value, capture, wildcard, `as`, `\|`, sequence and mapping patterns and guards | CST-06 |

Class patterns in `match`, such as `Point(x, y=0)`, are planned for 0.5
(CST-06).

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

## Matching values

`match` tries its cases in order and runs the first whose pattern matches
the subject and whose guard, the `if` after the pattern, is true. A
literal or a dotted name such as `Color.RED` matches a subject equal to
it, and `None`, `True` and `False` match the subject that is them. A name
captures the subject, `_` matches anything and binds nothing, `p as name`
binds what `p` matched, and `p | q` tries each alternative. When no case
matches, the statement does nothing.

<!-- typy-expect: accepted -->
```python
def describe(code: int | None) -> str:
    match code:
        case None:
            return "no code"
        case 200 | 201:
            return "ok"
        case 404 as missing:
            return f"missing {missing}"
        case other if other >= 500:
            return f"server {other}"
        case other:
            return f"code {other}"


for code in [200, None, 404, 503, 302]:
    print(describe(code))
```

A case narrows the subject as the test it stands for does: after
`case None:`, a later case sees an `int | None` subject as `int`. A
pattern that matches anything, a capture or `_`, must come last, as
CPython requires:

<!-- typy-expect: rejected E0103 at 3:10 -->
```python
code = 1
match code:
    case other:
        print(other)
    case 1:
        print("one")
```

### Sequence and mapping patterns

`[p, q]` or `(p, q)` matches a list, tuple or `range` of two items whose
items match `p` and `q`; the brackets do not decide the type. `*rest`
takes the items between the others as a new list, and `*_` skips them. A
`str` or `bytes` subject is not a sequence to a pattern, as in CPython.
`{"key": p}` matches a dict that holds the key with a value matching `p`,
and `**rest` takes the other items as a new dict.

<!-- typy-expect: accepted -->
```python
def describe(command: list[str]) -> str:
    match command:
        case []:
            return "nothing"
        case ["go", direction]:
            return f"going {direction}"
        case ["say", *words]:
            return " ".join(words)
        case [verb, *_]:
            return f"unknown {verb}"
        case _:
            return "not a command"


def size(settings: dict[str, int]) -> str:
    match settings:
        case {"width": width, "height": height, **others}:
            return f"{width}x{height} and {len(others)} more"
        case _:
            return "no size"


print(describe(["go", "north"]), describe(["say", "hi", "there"]))
print(size({"width": 3, "height": 4, "depth": 1}), size({}))
```

A pattern that no value of the subject's type can match is refused with
E0407, as a sequence pattern on a `str` or a tuple pattern of the wrong
length is. A sequence pattern on a union of several sequence types, such
as `list[int] | tuple[int, int]`, is planned for 1.0.

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
