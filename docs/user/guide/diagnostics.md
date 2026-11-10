# Diagnostics

TyPy reports every problem it finds in one run, each as a diagnostic.

## What TyPy 0.2 does

| Item | Rule |
| --- | --- |
| Every diagnostic has a stable code, a `file:line:column` place with columns counted in code points, and one sentence that says what is wrong | DIA-01 |
| A certain fix is shown as the corrected line under `help:` | DIA-02 |
| A diagnostic that refuses valid Python names the rule the program falls outside | DIA-03 |
| One root cause gives one error | DIA-04 |
| A code is never reused | DIA-06 |

JSON output comes in 0.3 (DIA-05), with warnings and `--deny-warnings`
(DIA-07). [Troubleshooting](../troubleshooting.md) explains every code.

## Example

A program with no problems checks cleanly:

<!-- typy-expect: accepted -->
```python
ratio: float = 1.0
print(ratio)
```

An `int` where a `float` is declared is refused, and the diagnostic shows
the corrected line, `ratio: float = 1.0`:

<!-- typy-expect: rejected E0402 at 1:16 -->
```python
ratio: float = 1
print(ratio)
```
