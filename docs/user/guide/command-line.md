# Command line

The `typy` command checks, builds and runs a program from its main file.

## What TyPy 0.2 accepts

| Command | What it does | Rule |
| --- | --- | --- |
| `typy check FILE` | Reports every diagnostic without generating code | CLI-03 |
| `typy build FILE` | Writes a native executable beside the file, or at `-o PATH` | CLI-01 |
| `typy run FILE [-- ARGS]` | Builds the program and runs it with `ARGS` | CLI-02 |
| `typy build --emit=STAGE FILE` | Writes one stage as text: `ast`, `typed-ast`, `tyir`, `tyir-mem`, `llvm-ir` or `asm`, or `obj` | CLI-04 |
| `--release`, `--release-small` | Optimise for speed or for size; the default is a debug build | CLI-05 |

The exit status of `typy` is 0 on success, 1 when the program has errors,
2 on wrong command usage and 101 on an internal compiler error (CLI-06).
`typy` reads no environment variable that changes its output, except
`TYPY_HOME` (CLI-11). A program reads its arguments through `sys.argv`,
which comes with imports in 0.3.

## Example

Save the program as `squares.py`, then run `typy run squares.py`:

<!-- typy-expect: accepted -->
```python
for number in range(1, 6):
    print(number, number ** 2)
```

`typy check` exits with status 1 and reports the problem when a program
is refused:

<!-- typy-expect: rejected E0205 at 1:7 -->
```python
print(undefined_name)
```
