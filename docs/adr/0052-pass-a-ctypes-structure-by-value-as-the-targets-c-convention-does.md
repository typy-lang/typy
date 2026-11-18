# ADR-0052: Pass a ctypes structure by value as the target's C convention does

## Status

Accepted

## Date

2026-11-17

## Context

ADR-0049 lays a `ctypes.Structure` out as C does and passes it by value as
the System V convention of x86-64 Linux does, and says another target
needs its own rules. AArch64 Linux follows AAPCS64, which passes the same
structures in other registers. TyPy calls C directly, without `libffi`
(ADR-0047), so it must lower each call as a C compiler lowers it.

clang 19, measured on 2026-11-17 by compiling the same C declarations for
`x86_64-linux-gnu` and `aarch64-linux-gnu` and reading the LLVM IR it
emits:

| C structure | x86-64 argument | AArch64 argument | AArch64 result |
| --- | --- | --- | --- |
| `{int, int}` | `i64` | `i64` | `i64` |
| `{double, double}` | `double, double` | `[2 x double]` | the structure, in `d0` and `d1` |
| `{float, float, float}` | `<2 x float>, float` | `[3 x float]` | the structure, in `s0` to `s2` |
| `{double, double, double, double}` | `byval` copy | `[4 x double]` | the structure, in `d0` to `d3` |
| `{int, double}` | `i32, double` | `[2 x i64]` | `[2 x i64]` |
| `{char, char, char}` | `i24` | `i64` | `i24` |
| `{__int128}` | `i64, i64` | `i128` | `i128` |
| five `float`s, 20 bytes | `byval` copy | pointer to a copy | `sret` |
| `{long, long, long}`, 24 bytes | `byval` copy | pointer to a copy | `sret` |

## Options considered

1. **Call through `libffi` on AArch64.** It knows every convention, but
   ADR-0047 chose direct calls for speed and for linking at build time, and
   a second call path would test differently on each target.
2. **One classification per target, as clang lowers it.** The x86-64 rules
   of ADR-0049 stay; AArch64 gets the rules of AAPCS64. Each is checked
   against clang's lowering of the same structures.
3. **Refuse structures by value on AArch64.** Programs that run on x86-64
   and under CPython would fail on one target only.

## Decision

Option 2. The layout of ADR-0049 holds on both targets, which are LP64.
Passing follows the target:

- **x86-64 Linux**, as ADR-0049 decided: up to 16 bytes in one or two
  registers, each eight bytes in a vector register when it holds only
  `float` or `double` values and in a general register otherwise, while
  the six general and eight vector registers last; anything else as a
  `byval` copy, and a large result through `sret`.
- **AArch64 Linux.** A structure of one to four fields, all `float` or all
  `double`, nested structures and arrays included, is a homogeneous
  floating-point aggregate: it passes as `[n x float]` or `[n x double]`
  and returns in `s0` to `s3` or `d0` to `d3`. Any other structure of up to
  16 bytes passes and returns as `i64`, `[2 x i64]` or, when its alignment
  is 16, `i128`, read without reading past the structure. A larger one
  passes as a pointer to a copy the caller makes, since the callee may
  change it, and returns through `sret`, which AArch64 passes in `x8`. The
  back end places each value in registers or on the stack, so TyPy counts
  no registers on AArch64.
- A structure of no bytes passes nothing on either target.
- ADR-0049 is superseded by this record.

## Consequences

- A program that passes structures by value builds and runs alike on both
  targets, and `crates/typy-cli/tests/structures.rs` runs for each.
- Apple's arm64 convention differs from AAPCS64 in how it packs values on
  the stack; it comes with macOS in 0.7.
- A third target adds a third classification, checked against clang in the
  same way.

## Related requirements

FFI-03, FFI-12, CLI-08, ADR-0047, ADR-0048, ADR-0049, ADR-0051.
