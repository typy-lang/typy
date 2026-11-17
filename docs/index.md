# TyPy 0.5 documentation

TyPy is an independent implementation of Python that compiles a strict,
typed subset of standard Python ahead of time to native executables and C
libraries.

## Documentation sections

- [Tutorial](tutorial/index.md): install TyPy, then check, build and run a
  first program.
- [Language reference](reference/index.md): what TyPy accepts and what it
  refuses, one page per area of the language, with the rule ID of each
  item.
- [Library reference](library/index.md): the built-in types, functions and
  exceptions, and the modules of the standard library TyPy provides.
- [Setup and usage](using/command-line.md): the `typy` command, its options
  and the project settings in `typy.toml`.
- [Extending and embedding](extending/c-interop.md): calling C libraries
  through `ctypes`, and building a module as a C library.
- [Diagnostics](diagnostics/index.md): how TyPy reports a problem, and
  [troubleshooting](diagnostics/troubleshooting.md) for every diagnostic
  code.

A program TyPy accepts is ordinary Python: it runs under CPython 3.12 and
later with the same output and exit status. A program TyPy cannot compile
with CPython's meaning is refused with a diagnostic that names the rule it
falls outside. Every Python sample in this documentation is checked by the
test suite against the current compiler, and an accepted sample is also run
under CPython and must print the same output.

## Engineering documents

- [Engineering documents](engineering.md): the research, requirements,
  models, architecture, decision records, design, test plan and
  development guide behind TyPy, with the status of each.

## Project information

- [Changelog](https://github.com/typy-lang/typy/blob/main/CHANGELOG.md):
  what changed in each release.
- [Reporting issues](https://github.com/typy-lang/typy/issues): report a
  problem with TyPy or with this documentation.
- [Contributing](https://github.com/typy-lang/typy/blob/main/CONTRIBUTING.md):
  how a change gets into TyPy.
- [Security policy](https://github.com/typy-lang/typy/blob/main/SECURITY.md):
  the supported versions, and how to report a vulnerability privately.
