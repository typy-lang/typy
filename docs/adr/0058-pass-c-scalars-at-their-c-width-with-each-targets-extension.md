# ADR-0058: Pass C scalars at their C width with each target's extension

## Status

Accepted

## Date

2026-11-18

## Context

ADR-0047 calls a C function directly and passes every integer and `bool`
argument as a 64-bit word, extended as `libffi` extends it, which Linux on
x86-64 and AArch64 accepts. Apple's arm64 convention differs from AAPCS64,
from Apple's "Writing ARM64 code for Apple platforms", read on 2026-11-18:

- The caller sign- or zero-extends an argument or result narrower than 32
  bits; under AAPCS64 the callee does.
- An argument passed on the stack takes its own size and alignment: ten
  `char` arguments after the registers run out take ten bytes, not ten
  eight-byte slots.
- Variadic arguments always go on the stack, in eight-byte slots.

LLVM's back end places a stack argument by its IR type, so a 64-bit word
for a C `char` takes eight bytes where the callee reads one. Clang marks a
narrow parameter or result `signext` or `zeroext` on Apple's arm64 and on
x86-64, and not on AArch64 Linux.

## Options considered

1. **Keep 64-bit words everywhere.** Wrong on Apple's arm64 as soon as an
   argument goes on the stack.
2. **Pass each scalar at its C width, with the attributes clang gives it
   on the target,** as ADR-0052 does for structures.
3. **Call through `libffi` on macOS.** A second call path, against
   ADR-0047's direct calls.

## Decision

Option 2. A `ctypes` scalar passes and returns as its C type: `i8`, `i16`,
`i32` or `i64` for the integers, `i1` for `c_bool`, `float` or `double`,
or a pointer. On the targets where clang marks them, x86-64 and Apple's
arm64, a signed integer narrower than 32 bits carries `signext` and an
unsigned one or `c_bool` `zeroext`, on a parameter and on a result. Clang 19
lowered `signed char f(signed char, unsigned char, short, unsigned short,
_Bool, int, long)` on 2026-11-18 as `signext i8 (i8 signext, i8 zeroext,
i16 signext, i16 zeroext, i1 zeroext, i32, i64)` for `x86_64-linux-gnu`
and `arm64-apple-macosx11.0.0`, and with no extension attribute for
`aarch64-linux-gnu`. This replaces ADR-0047's widening of arguments to
64-bit words; the rest of ADR-0047 stands. Variadic C functions stay
unsupported until FFI-10 in 1.0.

## Consequences

- C calls lower as clang lowers them on every target, so a `ctypes` call
  reads the same values on Apple's arm64 as on Linux.
- On Linux the change is in the IR alone: the values the callee reads stay
  the same.
- A test calls C functions with more narrow arguments than registers, so
  some go on the stack, on each target.

## Related requirements

FFI-01, FFI-02, FFI-12, FFI-13, ADR-0047, ADR-0052, ADR-0055.
