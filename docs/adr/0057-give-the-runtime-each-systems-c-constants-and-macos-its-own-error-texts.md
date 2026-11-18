# ADR-0057: Give the runtime each system's C constants and macOS its own error texts

## Status

Accepted

## Date

2026-11-18

## Context

The runtime has no standard library and declares the C functions it calls
itself (ADR-0006). Its constants are Linux's: `__errno_location`, the
octal `O_` flags, the place of `st_mode` in `struct stat`,
`RLIM_INFINITY` as all ones, and the errno numbers that pick an `OSError`
subclass. On macOS each differs, and some differ silently: Linux's
`O_APPEND` is macOS's `O_TRUNC`, so appending would empty the file.

Linux programs print glibc's error texts from a table (ADR-0019), because
musl's texts differ from those CPython prints. On macOS, CPython calls the
system's `strerror`, whose texts differ from glibc's for a few errors, such
as `ETIMEDOUT`, "Operation timed out", and an unknown number, "Unknown
error: N".

macOS facts, from the xnu-12377.121.6 headers and Libc-1752.120.2, checked
on 2026-11-18:

| Item | Linux | macOS |
| --- | --- | --- |
| errno accessor | `__errno_location` | `__error` |
| `O_APPEND`, `O_CREAT`, `O_TRUNC`, `O_EXCL`, `O_CLOEXEC` | `0o2000`, `0o100`, `0o1000`, `0o200`, `0o2000000` | `0x8`, `0x200`, `0x400`, `0x800`, `0x1000000` |
| `st_mode` | word 3 on x86-64, word 2 on AArch64 | `u16` at byte 4 of a 144-byte structure |
| `RLIM_INFINITY` | `u64::MAX` | `2^63 - 1` |
| `EAGAIN`, `EINPROGRESS`, `EALREADY` | 11, 115, 114 | 35, 36, 37 |
| `ECONNABORTED`, `ECONNRESET`, `ETIMEDOUT`, `ECONNREFUSED`, `ESHUTDOWN` | 103, 104, 110, 111, 108 | 53, 54, 60, 61, 58 |

## Options considered

1. **Read the constants from C headers when the runtime builds.** Needs a C
   compiler and the SDK in the runtime's build, which a static archive
   built by `cargo` otherwise does without.
2. **One module of constants for each operating system,** chosen with
   `cfg(target_os)`, each checked by tests on its own system.
3. **A table of macOS error texts like glibc's.** It would have to be
   generated on a Mac and kept in step with each macOS release.

## Decision

Option 2 for the constants. The error text of a number is glibc's from
ADR-0019's table on Linux, and on macOS the text the system's `strerror`
gives, since that is what CPython prints there; macOS's numbers pick the
`OSError` subclass there (EXC-19).

## Consequences

- `libc.rs` holds the declarations and constants of each system in its own
  module, and `os_error.rs` maps each system's numbers to the subclasses.
- A macOS program prints the error texts CPython prints on the same Mac,
  whichever macOS release it runs on.
- The runtime's unit tests of these constants run on each system's CI job.

## Related requirements

EXC-19, CLI-15, NFR-17, ADR-0006, ADR-0019, ADR-0055.
