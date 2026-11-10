# Compound statements

## What TyPy 0.2 accepts

| Item | Rule |
| --- | --- |
| `if`, `elif` and `else` | CST-01 |
| `while` and `for`, each with an optional `else` | CST-02 |
| `for` over `range`, `str`, `bytes`, `list`, `tuple`, `dict`, `set`, the views of a dict, and `enumerate`, `zip` and `reversed` of them | CST-03 |

`try` and `with` come in 0.3 (CST-04, CST-05), `match` in 0.5 (CST-06),
`for` over a class with `__iter__` in 0.3 (CST-08), and `iter` and `next`
in 0.3 (CST-09).

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
