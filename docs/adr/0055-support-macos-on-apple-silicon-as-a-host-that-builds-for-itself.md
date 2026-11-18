# ADR-0055: Support macOS on Apple silicon as a host that builds for itself

## Status

Accepted

## Date

2026-11-18

## Context

0.7 adds macOS on Apple silicon (CLI-15). macOS differs from Linux in its
object format, its linker, its C library and its C calling convention, and
linking for it needs Apple's SDK, whose licence keeps it to Apple hardware.
TyPy's development runs in a Debian dev container, where macOS cannot run.

Facts checked on 2026-11-18:

- Rust 1.99 and LLVM 23 build `aarch64-apple-darwin` for macOS 11.0 by
  default, and python-build-standalone's CPython for macOS arm64 targets
  11.0.
- GitHub's arm64 macOS runners for a public repository are free: `macos-15`
  (Xcode 16.4 by default) and `macos-26`, with 3 CPUs, 7 GB of memory and
  14 GB of disk. `macos-14` was retired on 2026-11-02.
- Every arm64 executable on macOS must be signed. Apple's linker signs it
  ad hoc; changing the file afterwards breaks the signature.

## Options considered

1. **Build for macOS from Linux too.** Needs Apple's SDK on a Linux machine,
   which its licence does not allow.
2. **Support macOS as a host that builds for itself,** with its own CI job
   and release archive.
3. **Leave macOS until after 1.0.** Keeps 0.7 small, but the release plan
   gives macOS to 0.7.

## Decision

Option 2.

- **Target.** `aarch64-apple-darwin`, given to LLVM as
  `arm64-apple-macosx11.0.0` with the CPU `apple-m1`, the first Apple
  silicon, so a program runs on every Apple silicon Mac from macOS 11.
- **Hosts.** A macOS host builds for `aarch64-apple-darwin`; a Linux host
  builds for the Linux targets and refuses the macOS one, listing its own.
  A macOS host does not build for Linux in 0.7.
- **Tools.** The C driver is `cc` of Xcode or its Command Line Tools,
  which links with Apple's linker (ADR-0056). macOS has no `objcopy`.
- **Python names.** `sys.platform` is `darwin` in a program built for
  macOS (LIB-22), so checking takes the target from 0.7.
- **Development.** On a Mac, natively: rustup, the pinned LLVM of ADR-0054,
  Xcode's Command Line Tools and uv's CPython oracles; `make EXEC=` runs
  each target without the container. Linux development does not change.
- **CI.** A `macos-15` job runs the unit, integration and conformance
  tests natively against CPython for macOS (NFR-17).
- **Release.** A `macos-15` job builds
  `typy-X.Y.Z-aarch64-apple-darwin.tar.gz`, holding the runtime archive of
  `aarch64-apple-darwin` only. It is signed ad hoc by the linker and not
  notarized (OI-29), so the documentation shows how to clear the quarantine
  a browser download carries.

## Consequences

- macOS changes are built and tested on GitHub's runners or on a Mac, not
  in the dev container.
- The Linux release archives keep the runtime archives of the Linux targets
  alone, since a Linux host does not build for macOS.
- A Mac user downloading with a browser runs
  `xattr -d com.apple.quarantine` on the archive once, until a notarized
  archive exists.

## Related requirements

CLI-08, CLI-15, CLI-16, LIB-22, EXC-19, FFI-13, NFR-17, OI-29, ADR-0051,
ADR-0054, ADR-0056, ADR-0057, ADR-0058.
