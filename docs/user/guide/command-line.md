# Command line

The `typy` command checks, builds and runs a program from its main file.

## What TyPy accepts

| Command | What it does | Rule |
| --- | --- | --- |
| `typy check FILE` | Reports every diagnostic without generating code | CLI-03 |
| `typy build FILE` | Writes a native executable beside the file, or at `-o PATH` | CLI-01 |
| `typy run FILE [-- ARGS]` | Builds the program and runs it with `ARGS` | CLI-02 |
| `typy explain CODE` | Prints what a diagnostic code means, a program that gives it and the corrected program | CLI-10 |
| `typy build --emit=STAGE FILE` | Writes one stage as text: `ast`, `typed-ast`, `tyir`, `tyir-mem`, `llvm-ir` or `asm`, or `obj` | CLI-04 |
| `--release`, `--release-small` | Optimise for speed or for size; the default is a debug build | CLI-05 |
| `--deny-warnings` | Fails with status 1 when there are warnings | DIA-07 |
| `--error-format=json` | Writes diagnostics as one JSON object per line | DIA-05 |
| `--python-floor VERSION` | Sets the Python floor for this command, over `typy.toml` | CLI-07, CLI-12 |

The exit status of `typy` is 0 on success, 1 when the program has errors,
2 on wrong command usage and 101 on an internal compiler error (CLI-06).
`typy` reads no environment variable that changes its output, except
`TYPY_HOME` (CLI-11). A program reads its arguments through `sys.argv`,
which comes with imports in 0.3.

## Project settings in typy.toml

A file `typy.toml` beside the main file holds the project's settings.
TyPy reads it once, when a command starts, and a flag on the command line
wins over the same setting in the file (CLI-07). Without the file, the
defaults apply.

| Setting | Value | Default | Rule |
| --- | --- | --- | --- |
| `python-floor` | The oldest CPython version the program supports: `"3.12"`, `"3.13"`, `"3.14"` or `"3.15"`, written as a string | `"3.12"` | CLI-12 |
| `module-paths` | Directories under the project root, named relative to it, where imports look for modules after the main file's directory, in order, as CPython looks in `PYTHONPATH` | `[]` | MOD-01 |

The floor decides which syntax the program may use and which CPython
versions it promises to match (ADR-0008); `lazy import` needs `"3.15"`.
A project that keeps shared modules in `lib/` and uses lazy imports
writes:

```toml
python-floor = "3.15"
module-paths = ["lib"]
```

and CPython runs it the same way with `PYTHONPATH=lib`. A setting TyPy
does not know, a value of the wrong type, a module path outside the
project, and a file that is not valid TOML are E0107: TyPy names the line
and exits with status 2 before it reads any module.

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
