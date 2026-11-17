# Standard Python

TyPy adds nothing to Python. It accepts only programs that CPython also
runs, and an accepted program behaves as CPython does.

## What TyPy accepts

| Item | Rule |
| --- | --- |
| Source that every supported CPython version from the program's floor up parses; the floor is 3.12 | STD-01 |
| Programs that run alike under TyPy and CPython, on every input where CPython finishes without an uncaught exception | STD-02 |
| Behaviour the Language Reference or the library documentation defines; `id()` values, hash values and the order of sets of objects are not promised | STD-03 |
| A program that uses a feature TyPy cannot compile with CPython's meaning is refused, never compiled differently | STD-04 |
| Module-level statements run in order, and `__name__` is `"__main__"` | STD-06 |
| No module, function, type, decorator or syntax of TyPy's own | STD-07 |

An uncaught exception ends the program as in CPython: a traceback on
standard error, the last line `ValueError: ...` as CPython writes it, and
exit status 1.

## Example

<!-- typy-expect: accepted -->
```python
def main() -> None:
    total = 0
    for value in range(1, 11):
        total += value * value
    print("sum of squares:", total)


if __name__ == "__main__":
    main()
```

A feature TyPy cannot compile with CPython's meaning is refused. Here a
function is defined only when a test holds, so whether it exists depends
on the run:

<!-- typy-expect: rejected E0202 at 2:5 -->
```python
if True:
    def helper() -> None:
        pass
```
