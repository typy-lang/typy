# ADR-0056: Link for Mach-O with Apple's linker and name libraries as macOS does

## Status

Accepted

## Date

2026-11-18

## Context

TyPy links through the C driver with options of GNU ld, and builds
libraries with `objcopy`, `ar rcsD`, a version script and an ELF soname
(ADR-0044, ADR-0045, ADR-0047). None of these exists on macOS. Apple's
linker since Xcode 15, ld-prime, takes its own options; macOS has no static
executables; `libm` is part of `libSystem`; BSD `ar` has no `D` modifier;
and a linked binary must not be changed afterwards, or its ad hoc signature
breaks. Facts checked on 2026-11-18 against the ld-prime manual of Xcode
27.3 and ld64-957.1.

## Options considered

1. **One set of options for both.** Not possible: the flags, the export
   lists and the library names differ.
2. **A Mach-O path beside the ELF one,** chosen by the target.
3. **Link with lld's Mach-O flavour on every host.** Needs Apple's SDK all
   the same, and is not the linker macOS developers use.

## Decision

Option 2, Apple's linker through `cc`.

| Step | Linux (ADR-0044 to 0047) | macOS |
| --- | --- | --- |
| Executable | `-Wl,--gc-sections`, `-lm`, `-static` for musl | `-arch arm64 -mmacosx-version-min=11.0`, `-Wl,-dead_strip`; `libm` comes with `libSystem`; no static executables |
| Release build | `-Wl,--strip-debug` | `-Wl,-S` |
| Debug build | DWARF in the executable | DWARF read from the object by `dsymutil` into `NAME.dSYM` beside the executable, since the linker leaves it in the object, which TyPy deletes |
| A `typy.toml` library, dynamic | `-l:FILE` | `-lNAME` for `libNAME.dylib`, the name a `ctypes` declaration gives |
| A `typy.toml` library, static | `-l:BASE.a` | the archive's path |
| Static library | `cc -r`, `objcopy --keep-global-symbols`, `ar rcsD` | `ld -r -exported_symbols_list`, then `libtool -static -D` |
| Shared library | `-shared`, a version script, `-soname` | `-dynamiclib`, `-exported_symbols_list`, `-install_name`, `-compatibility_version`, `-current_version` |
| Names (CLI-13, CLI-16) | `libNAME.so.X.Y`, soname `libNAME.so.X`, `libNAME.X.Y.a` | `libNAME.X.dylib` with that install name and the versions `X.0` and `X.Y`, a link `libNAME.dylib`, and `libNAME.a` |

An export list names each symbol as Mach-O spells it, with a leading
underscore. `ZERO_AR_DATE=1` is set for each tool, so an archive and a
debug map carry no times (NFR-06). TyPy never changes a binary after the
linker wrote it.

## Consequences

- `linker.rs` and `library_link.rs` choose their steps by the target's
  object format, and the binding module of ADR-0046 names the `.dylib`.
- A debug build on macOS writes a `.dSYM` directory beside the executable,
  which a debugger reads.
- A macOS program cannot be linked statically, so the musl target has no
  macOS counterpart.

## Related requirements

CLI-05, CLI-13, CLI-15, CLI-16, FFI-04, NFR-06, ADR-0044, ADR-0045,
ADR-0046, ADR-0047, ADR-0055.
