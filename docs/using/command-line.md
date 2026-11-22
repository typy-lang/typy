# Command line

The `typy` command checks, builds and runs a program from its main file.

## What TyPy accepts

| Command | What it does | Rule |
| --- | --- | --- |
| `typy check FILE` | Reports every diagnostic without generating code | CLI-03 |
| `typy build FILE` | Writes a native executable beside the file, or at `-o PATH` | CLI-01 |
| `typy run FILE [-- ARGS]` | Builds the program and runs it with `ARGS` | CLI-02 |
| `typy explain CODE` | Prints what a diagnostic code means, a program that gives it and the corrected program | CLI-10 |
| `typy print WHAT` | Writes what `typy` would use: `targets`, `target-cpus`, `host`, `paths`, `settings` or `version`; see [Asking typy what it uses](#asking-typy-what-it-uses) | CLI-25 |
| `typy NAME [ARGS]` | Runs the program `typy-NAME` found on `PATH`, for a NAME that is not a command of `typy`; see [Commands of other tools](#commands-of-other-tools) | CLI-32 |
| `typy build --lib KIND FILE` | Writes a `static` or `shared` C library of the functions the module's `__all__` lists, and its C header; see [C interoperability](../extending/c-interop.md) | CLI-09 |
| `typy build --target TRIPLE FILE` | Builds for another target the host builds for, with that target's C toolchain: on Linux `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl` or `aarch64-unknown-linux-gnu`, and on macOS `aarch64-apple-darwin`; `typy check` takes `--target` too | CLI-08, CLI-14 |
| `typy build --emit=STAGE FILE` | Writes one stage as text: `ast`, `typed-ast`, `tyir`, `tyir-mem`, `llvm-ir` or `asm`, or `obj` | CLI-04 |
| `--release`, `--release-small` | Optimise for speed or for size, without debug information; the default is a debug build, which keeps it | CLI-05 |
| `--deny-warnings` | Fails with status 1 when there are warnings | DIA-07 |
| `--error-format=json` | Writes diagnostics as one JSON object per line | DIA-05 |
| `--python-floor VERSION` | Sets the Python floor for this command, over `typy.toml` | CLI-07, CLI-12 |
| `--timings` | Writes how long each stage took to standard error: `load`, `check`, `lower`, `generate`, `link` and the `total` | CLI-22 |
| `-v`, `--verbose` | Writes each command `typy` runs to standard error before it runs it, as a shell would read it: the C linker with its arguments, and for `typy run` the program | CLI-23 |
| `--save-temps` | Keeps a build's object file beside its output, named after it up to its first dot: `app.o` for `app`, `libtools.o` for `libtools.so.1` | CLI-24 |

The exit status of `typy` is 0 on success, 1 when the program has errors,
2 on wrong command usage and 101 on an internal compiler error (CLI-06).
`typy` reads no environment variable that changes its output, except
`TYPY_HOME` (CLI-11). A program reads its arguments through `sys.argv`,
which comes with imports in 0.3.

## Asking typy what it uses

`typy print` writes one fact a line, as `typy.toml` writes its settings, so
a person or a script can read it:

```sh
typy print targets
typy print target-cpus --target aarch64-unknown-linux-gnu
typy print paths
typy print settings app.py --python-floor 3.15
typy print version
```

`targets` lists every supported target, marks the host, and says which
need another host to build. `target-cpus` lists a target's processors: its
baseline first, which every build uses unless it asks for another,
`native` for the machine `typy` runs on, and every processor LLVM knows.
`paths` names the compiler, the runtime archives it links for a target,
for debug and `--release` builds and for `--release-small`, and its cache.
`settings` writes the settings a command would use for the project of a
main file, `typy.toml` and the command line together, and refuses a
`typy.toml` that `typy check` refuses. `version` names the versions of
`typy`, its runtime and LLVM, and the Python floors.

## Seeing what a build does

`--timings` and `-v` show where a build spends its time and what it runs:

```sh
typy build app.py --timings -v --save-temps
```

`-v` writes the linker's command as `typy` runs it, which names the
temporary object file it links; `--save-temps` keeps that object as
`app.o` beside `app`, and the command links it the same way. A `typy run`
that takes its build from the cache runs no build, so it writes the
timings of loading and checking only, and keeps no object file.

## Commands of other tools

`typy` is the compiler: `check`, `build`, `run`, `explain` and `print`.
Any other command runs a program of its own, as `git` and `cargo` run
theirs: `typy fmt --check` runs `typy-fmt --check`, the first program named
`typy-fmt` on `PATH` that may run. Its output, its input and its exit status
are the program's own, and `TYPY` names the `typy` that ran it, so the tool
builds with the same compiler. A command that is not a word, or that no
program on `PATH` answers, is wrong usage, exit status 2. Running a
command this way trusts the programs on your `PATH`, as running `git` or
`cargo` commands does (ADR-0105).

## Building for another target

`typy build` builds for the machine it runs on unless `--target` names
another target. A Linux machine builds for every Linux target, and a Mac
for macOS alone, since linking for macOS needs Apple's tools, which run
only on a Mac; see [Using TyPy on macOS](macos.md). Another triple is a
usage error that lists the host's targets. On an x86_64 Linux machine,
this builds `app` for 64-bit Arm Linux:

```console
typy build app.py --target aarch64-unknown-linux-gnu
```

TyPy generates the code itself and links it with the C toolchain of the
target. For the host's own architecture that is `cc`, `ar` and `objcopy`,
and `musl-gcc` for `x86_64-unknown-linux-musl`. For the other architecture
it is the toolchain named after the target's GNU triple, such as
`aarch64-linux-gnu-gcc`, which Debian and Ubuntu package with the target's
C library:

```console
sudo apt install gcc-aarch64-linux-gnu libc6-dev-arm64-cross
```

A missing tool is E0701, which names the program TyPy looked for and the
target. A table `[target."TRIPLE"]` in `typy.toml` names other tools for a
target, each by its name, which is looked up on `PATH`, or its absolute
path:

```toml
[target."aarch64-unknown-linux-gnu"]
linker = "/opt/cross/bin/aarch64-linux-gnu-gcc"
ar = "/opt/cross/bin/aarch64-linux-gnu-ar"
objcopy = "/opt/cross/bin/aarch64-linux-gnu-objcopy"
```

The program runs on the target's machines, or on this one under an emulator
such as `qemu-aarch64` from the `qemu-user` package:

```console
qemu-aarch64 -L /usr/aarch64-linux-gnu ./app
```

`typy run` builds for the host only, since a program for another
architecture does not run on it. `typy check --target` checks for the
target: `sys.platform` is the name of its operating system, and the C
types have the same sizes on every target.

## Project settings in typy.toml

A file `typy.toml` beside the main file holds the project's settings.
TyPy reads it once, when a command starts, and a flag on the command line
wins over the same setting in the file (CLI-07). Without the file, the
defaults apply.

| Setting | Value | Default | Rule |
| --- | --- | --- | --- |
| `python-floor` | The oldest CPython version the program supports: `"3.12"`, `"3.13"`, `"3.14"` or `"3.15"`, written as a string | `"3.12"` | CLI-12 |
| `module-paths` | Directories under the project root, named relative to it, where imports look for modules after the main file's directory, in order, as CPython looks in `PYTHONPATH` | `[]` | MOD-01 |
| `library-paths` | Directories under the project root where the linker looks for C libraries first, as `-L`; see [C interoperability](../extending/c-interop.md#linking-a-library) | `[]` | FFI-04 |
| `[libraries."NAME"]` | `link = "static"` or `"dynamic"` for the C library a `ctypes` declaration names `NAME` | dynamic | FFI-04 |
| `version` | The version of a library whose module has no `__version__`, such as `"5.0"`, whose release names its files; see [C interoperability](../extending/c-interop.md#naming-a-library-by-its-version) | none | CLI-13 |
| `[allocator]` | The C functions the program allocates its memory with, and their library; see [Memory](../reference/memory.md#choosing-the-allocator) | `malloc`, `realloc` and `free` | MEM-08 |
| `[target."TRIPLE"]` | The C tools that build for a target, in place of those TyPy runs: `linker`, the C compiler driver, `ar` and `objcopy`, each a program's name or absolute path; see [Building for another target](#building-for-another-target) | the host's `cc`, `ar` and `objcopy`, or those named after the target's GNU triple | CLI-14 |

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
project, a wrong `[allocator]` table, and a file that is not valid TOML are
E0107: TyPy names the line and exits with status 2 before it reads any
module.

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
