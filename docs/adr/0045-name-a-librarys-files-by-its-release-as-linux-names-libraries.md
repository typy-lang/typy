# ADR-0045: Name a library's files by its release as Linux names libraries

## Status

Accepted

## Date

2026-11-17

## Context

ADR-0044 names a library `libNAME.a` or `libNAME.so` after its module. A
library that is installed beside other versions of itself, or that a
program loads by its version, needs the version in its file names, and
CLI-13 asks for it.

Two facts shape the design. A C linker finds a library given `-lNAME` only
as `libNAME.so` or `libNAME.a`, so a file named only with its version
cannot be linked by its name. And a Python module keeps its version, when
it keeps one at all, in `__version__`, which many modules do not define,
while a TyPy project already has `typy.toml` for what is not in the
source.

## Options considered

For the names:

1. **The version before the suffix**, `libNAME.5.0.so` and
   `libNAME.5.0.a`, with or without links of the plain names.
2. **The Linux convention**: the shared library `libNAME.so.5.0`, whose
   soname is `libNAME.so.5`, with links `libNAME.so.5` and `libNAME.so`,
   as distributions ship libraries, so a program linked with release 5.0
   loads any later release 5.x; and a static library `libNAME.5.0.a` with
   a link `libNAME.a`.

For where the version comes from:

1. **`__version__` of the module** only.
2. **`version` in `typy.toml`** only.
3. **`__version__`, or else `version` in `typy.toml`**, which must agree
   when both are given.

## Decision

TyPy uses option 2 for the names and option 3 for the version.

- **The release.** The names carry the release of the version, as PEP 440
  reads it: the numbers and dots it starts with, after an optional `v`, so
  `5.0` of `5.0rc1`. The major release is its first number.
- **The version.** A library build reads `__version__` when the module
  binds it once, to a string literal; otherwise `version` in `typy.toml`.
  A version that does not start with a number, a `__version__` bound
  otherwise, and a `__version__` that differs from `version` are E0429
  under CLI-13; a `version` in `typy.toml` that does not start with a
  number is E0107. Without either, the names have no version.
- **The files**, beside the main file:

  | Kind | Library | Soname | Links |
  | --- | --- | --- | --- |
  | Shared, release `X.Y` | `libNAME.so.X.Y` | `libNAME.so.X` | `libNAME.so.X`, `libNAME.so` |
  | Shared, no release | `libNAME.so` | `libNAME.so` | none |
  | Static, release `X.Y` | `libNAME.X.Y.a` | none | `libNAME.a` |
  | Static, no release | `libNAME.a` | none | none |

  Each link names the library's file, and replaces whatever had its name,
  so the links of a new release move to it.
- **`-o PATH`** writes the library at that path alone, with no links, and
  a shared library's soname is that file's name.

## Consequences

- `-lNAME` links the newest release a directory holds, and a program
  linked with one major release keeps loading it after a minor one is
  installed.
- A module without `__version__` is versioned from `typy.toml` with no
  change to its source.
- A release whose major number changes gets a soname of its own, so
  programs linked with the one before keep loading it until they are
  linked again.

## Related requirements

CLI-09, CLI-13, ADR-0044.
