# TyPy user documentation

TyPy compiles a strict, typed subset of standard Python ahead of time to a
native executable. A program TyPy accepts is ordinary Python: it runs under
CPython 3.12 and later with the same output and exit status. A program
TyPy cannot compile with CPython's meaning is refused with a diagnostic
that names the rule it falls outside.

This documentation covers TyPy 0.3.

| Page | What it covers |
| --- | --- |
| [Getting started](getting-started.md) | Installing TyPy and checking, building and running a first program |
| [Language guide](guide/index.md) | One page per area of the language: what 0.3 accepts, what it refuses and what is planned |
| [Troubleshooting](troubleshooting.md) | Every diagnostic code, what causes it and how to change the program |
| [Changelog](../../CHANGELOG.md) | What changed in each release |

Every Python sample on these pages is checked by the test suite against
the current compiler, and an accepted sample is also run under CPython and
must print the same output.
