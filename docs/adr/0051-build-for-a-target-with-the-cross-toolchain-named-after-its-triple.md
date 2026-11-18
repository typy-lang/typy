# ADR-0051: Build for a target with the cross toolchain named after its triple

## Status

Accepted

## Date

2026-11-17

## Context

CLI-08 asks `typy build --target TRIPLE` to build for every supported
target, and section 2.3 of the specification adds Linux AArch64 with glibc
in 0.6. macOS AArch64 moves to 0.7: linking for it needs Apple's SDK, which
its licence keeps to Apple hardware, so macOS is a port of the compiler to
a new host rather than one more target.

Most of the path exists. `Target` travels from the build request through
code generation, the lookup of the runtime archive, the linker and the
cache key of `typy run`. What is missing:

- `--target` on the command line, and any AArch64 triple.
- LLVM is initialised for x86 alone, with the CPU `x86-64`, although the
  LLVM TyPy links has the AArch64 back end.
- The linker table names `cc` and `musl-gcc` only, and a library build runs
  the host's unprefixed `objcopy` and `ar`, which do not edit another
  architecture's objects on every distribution.
- The runtime has one line of x86 assembly, the read of the stack pointer
  in its stack check. Its `stat` layout, `open` flags and errno numbers are
  already those of AArch64 Linux.
- The ABI questions have decisions of their own: structures by value in
  ADR-0052 and the runtime's own calls in ADR-0053.

## Options considered

1. **clang and lld with a sysroot.** One driver for every target, given
   `--target` and the path of the target's C library. Every user names a
   sysroot, and clang is not installed on every Linux host.
2. **The GNU cross toolchain named after the triple.** For a target other
   than the host's, TyPy runs `aarch64-linux-gnu-gcc`,
   `aarch64-linux-gnu-ar` and `aarch64-linux-gnu-objcopy`, as Debian and
   Ubuntu package them, and `typy.toml` can name other tools for a target.
   The distribution's packages bring the C library, the start files and
   the binutils in one install.
3. **Ship a linker and C libraries with TyPy.** Builds for any target with
   nothing installed, but adds hundreds of megabytes per target to the
   archive, and the C libraries' licences to the release.

## Decision

Option 2.

- **Targets.** 0.6 supports `x86_64-unknown-linux-gnu`,
  `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-gnu`. `typy build`
  and `typy check` take `--target TRIPLE`; another triple is a usage error,
  status 2, that lists these three (UC-08 2a). `typy run` builds for the
  host only, since another architecture's program does not run there.
- **Host.** The compiler's own architecture and C library give the host's
  target, so an AArch64 `typy` builds for AArch64 by default.
- **Code.** LLVM is initialised for x86 and AArch64. The CPU is each
  architecture's baseline, `x86-64` and `generic` (Armv8.0-A), so a program
  runs on every machine of its architecture and the output does not depend
  on the machine that builds it (NFR-06).
- **Tools.** The host's own target uses `cc`, `ar` and `objcopy`, and
  `musl-gcc` for musl as before. Another target uses the tools whose names
  start with its GNU triple: `aarch64-linux-gnu-` from x86_64, and
  `x86_64-linux-gnu-` from AArch64. A table `[target."TRIPLE"]` in
  `typy.toml` may set `linker`, `ar` and `objcopy` for a target (CLI-14). A
  tool that is not found is E0701, which names the tool and the target
  (UC-08 3a).
- **Runtime archives.** `scripts/build-runtime.sh` builds the runtime for
  each target, which needs only the Rust target, since a static library is
  not linked. Every compiler archive holds the runtime archives of all
  three targets, so either host builds for the other.
- **Release.** Each release has a compiler archive for each host,
  `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`, the AArch64
  one built on an arm64 runner.
- **Python names.** `sys.platform` is `linux` and the `ctypes` sizes are
  LP64 on all three targets, so checking does not depend on the target in
  0.6; macOS makes `sys.platform` depend on it in 0.7.

## Consequences

- A developer installs the distribution's cross package once, such as
  `gcc-aarch64-linux-gnu` on Debian, to build for the other architecture
  (A-01).
- An AArch64 program built on an x86_64 host runs there only under an
  emulator such as `qemu-aarch64`, which the test suite uses.
- `aarch64-unknown-linux-musl` is not a target in 0.6: Debian and Ubuntu
  package musl for the host's own architecture only.
- The golden files of LLVM IR and assembly name their target, and the
  runtime size checks of NFR-02 run for each target.

## Related requirements

CLI-08, CLI-14, NFR-06, NFR-16, UC-08, ADR-0006, ADR-0044, ADR-0052,
ADR-0053.
