# ADR-0054: Build and pin LLVM for every host from its sources

## Status

Accepted

## Date

2026-11-18

## Context

TyPy links LLVM 23 statically (C-03). Up to 0.6 the dev image installs it
from apt.llvm.org, which publishes dated snapshot builds of 23.1.x and drops
old ones. Two problems follow:

- A release built in CI is not byte for byte the build a developer makes of
  the same commit, because the two images hold different snapshots: the
  0.5.0 archive rebuilt locally differed from the published one (OI-28,
  NFR-06).
- On 2026-11-18 three CI runs failed while apt.llvm.org refused the
  runners' connections, retries included.

0.7 adds macOS on Apple silicon as a host (ADR-0055), and no pinned LLVM 23
for it is usable as it ships. Checked on 2026-11-18:

| Source | What stops it |
| --- | --- |
| Homebrew `llvm` 23.1.6 | No way to install an exact patch release; bottles built for macOS 15 and later; links Homebrew's `zstd` and `z3` libraries, so a shipped `typy` would need Homebrew |
| Official `LLVM-23.1.x-macOS-ARM64` archive | Missing for 23.1.0 and 23.1.6; its static libraries are LLVM bitcode, which Apple's linker cannot read for LLVM 20 and later (llvm-project issue 133580); its `llvm-config --system-libs` prints a path on the build machine, on which llvm-sys panics (issue 240026); 0.87 to 1.57 GB |
| conda-forge `llvmdev` 23.1.5 | Built against conda's own `libc++` and `zstd` |

## Options considered

1. **Keep apt.llvm.org on Linux and take Homebrew on macOS.** No pin on
   either, a macOS 15 minimum and Homebrew libraries in the shipped
   compiler.
2. **Use the official release archives.** Bitcode libraries, a broken
   `llvm-config`, and no archive for the current patch release.
3. **Use conda-forge's build.** Pinned, but the compiler would link conda's
   C++ library.
4. **Build LLVM from its sources in a workflow of TyPy's own,** for each
   host, and keep each build pinned by its SHA-256.

## Decision

Option 4.

- **What is built.** LLVM 23.1.6 from the signed source archive of tag
  `llvmorg-23.1.6`, with CMake: `Release`, static libraries, the targets
  `AArch64` and `X86`, assertions off, and no `zlib`, `zstd`, `libxml2`,
  `z3` or terminal libraries, so the build depends on the C and C++
  libraries of its host alone. It installs the libraries, the headers and
  `llvm-config`.
- **For which hosts.** `x86_64-unknown-linux-gnu` and
  `aarch64-unknown-linux-gnu`, each built in the Debian 13 base of the dev
  image so its glibc is the image's, and `aarch64-apple-darwin`, built on a
  `macos-15` runner for macOS 11.0, the minimum of ADR-0055.
- **Where it is kept.** The workflow `llvm.yml`, run by hand by a
  maintainer, writes each build as `llvm-23.1.6-HOST.tar.zst` to a release
  `llvm-23.1.6` of the repository, marked as a pre-release so it is never
  the latest release. `.devcontainer/llvm.lock` records each archive's name
  and SHA-256.
- **Who uses it.** The Dockerfile downloads the archive of its
  architecture and checks it against `llvm.lock` in place of apt.llvm.org;
  the macOS jobs do the same, keeping the archive in GitHub's Actions cache
  by its SHA-256; llvm-sys finds it through `LLVM_SYS_231_PREFIX`.
- **A new LLVM release** is a deliberate change: run `llvm.yml` for the new
  version, then change `llvm.lock` in one pull request.

## Consequences

- A CI build and a developer's build of one commit use the same LLVM bytes,
  which settles OI-28, and CI no longer depends on apt.llvm.org.
- A build of LLVM takes about one to two and a half hours per host. It runs
  once per LLVM version, not per pull request.
- The first dev image built after this change downloads the Linux archive
  once, a few hundred MB, in place of apt.llvm.org's packages.
- Only the job that writes the release assets may write to the repository,
  and only when a maintainer runs it.

## Related requirements

C-03, NFR-06, CLI-15, OI-28, ADR-0006, ADR-0051, ADR-0055.
