# Diagnostics

TyPy reports every problem it finds in one run, each as a diagnostic.

## What TyPy does

| Item | Rule |
| --- | --- |
| Every diagnostic has a stable code, a `file:line:column` place with columns counted in code points, and one sentence that says what is wrong | DIA-01 |
| A certain fix is shown as the corrected line under `help:` | DIA-02 |
| A diagnostic that refuses valid Python names the rule the program falls outside | DIA-03 |
| One root cause gives one error | DIA-04 |
| `--error-format=json` writes each diagnostic as one JSON object per line | DIA-05 |
| A code is never reused | DIA-06 |

[Troubleshooting](../troubleshooting.md) explains every code.

## Diagnostics as JSON

An editor or a CI job runs `typy check app.py --error-format=json` and
reads one object per line from standard error. Every object has the same
keys, with `null` where a diagnostic has nothing, and lines and columns
count from 1, with columns in characters:

```json
{"code":"E0402","severity":"error","message":"`ratio` was declared as float
and is assigned an int","file":"app.py","line":1,"column":16,"end_line":1,
"end_column":17,"rule":"TYP-09","help":null,"labels":[{"line":1,"column":1,
"text":"the variable"}],"fix":{"line":1,"column":16,"end_line":1,
"end_column":17,"replacement":"1.0"}}
```

The object is one line; it is wrapped here to fit the page.

| Key | Value |
| --- | --- |
| `code`, `severity`, `message` | The code, `"error"` or `"warning"`, and the sentence |
| `file`, `line`, `column`, `end_line`, `end_column` | Where the problem is |
| `rule` | The rule it enforces, such as `"TYP-09"` |
| `help` | A hint, when there is one |
| `labels` | Other places, each with `line`, `column` and `text` |
| `fix` | A certain fix: the span to replace and its `replacement` |

The exit status is the same as with text.

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
