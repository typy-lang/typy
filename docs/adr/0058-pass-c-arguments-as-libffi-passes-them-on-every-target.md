# ADR-0058: Pass C arguments as libffi passes them on every target

## Status

Accepted

## Date

2026-11-18

## Context

ADR-0047 calls a C function directly and passes every integer and `bool`
argument as a 64-bit word, widened by its sign or with zeros, as `libffi`
widens it for CPython's `ctypes`. A C function whose type is wider than its
declaration therefore reads what it reads under CPython: `labs` declared
with a `c_int8` argument returns 1 for -1, which a conformance test of
FFI-03 checks. A result takes its C type, and TyPy extends it itself.

Apple's arm64 convention differs from AAPCS64, from Apple's "Writing ARM64
code for Apple platforms", read on 2026-11-18:

- The caller sign- or zero-extends an argument or result narrower than 32
  bits; under AAPCS64 the callee does.
- An argument passed on the stack takes its own size and alignment: ten
  `char` arguments after the registers run out take ten bytes, not ten
  eight-byte slots.
- Variadic arguments always go on the stack, in eight-byte slots.

`libffi` follows both conventions. Its AArch64 port writes an integer
argument that goes in a register as a 64-bit word widened by its type, on
Linux and on macOS alike; one that goes on the stack takes an eight-byte
slot on Linux and its own size on macOS. LLVM's back end places a stack
argument by its IR type, so a 64-bit word for a C `char` takes eight bytes
where a callee on macOS reads one.

Clang 19.1.7 lowered `signed char f(signed char, unsigned char, short,
unsigned short, _Bool, int, long)` on 2026-11-18 as `signext i8 (i8
signext, i8 zeroext, i16 signext, i16 zeroext, i1 zeroext, i32, i64)` for
`x86_64-linux-gnu` and `arm64-apple-macosx11.0.0`, and with no extension
attribute for `aarch64-linux-gnu`. In the same dev image, a call through
that lowering of `labs` declared with a `signed char` parameter returned -1
for -1, where CPython 3.12's `ctypes` returns 1: clang extends an argument
to 32 bits only. A `zeroext` or `signext` result also lets LLVM skip the
caller's own extension, which a C function wider than its declared result
does not make.

An export of a library (ADR-0044) is the other side: C code calls TyPy. Its
wrapper takes a `bool` as `i8 zeroext` on every target, so on AArch64 Linux
LLVM assumes an extension that the caller does not make there.

## Options considered

1. **Keep 64-bit words everywhere.** Wrong on Apple's arm64 as soon as an
   argument goes on the stack.
2. **Pass every scalar at its C width, with the attributes clang gives it
   on the target.** Matches a C caller, but a C function wider than its
   declaration then reads other values than under CPython, on Linux too.
3. **Pass each argument as `libffi` passes it on the target:** a 64-bit
   word in a register, and on Apple's arm64 the C width on the stack.
4. **Call through `libffi` on macOS.** A second call path, against
   ADR-0047's direct calls.

## Decision

Option 3 for calls of C functions, and clang's lowering for exports.

- **Arguments of a call.** An integer or `c_bool` argument that goes in a
  general register passes as a 64-bit word, widened by its sign or with
  zeros, on every target. On Apple's arm64, one that goes on the stack,
  once `x0` to `x7` are taken, passes at its C width: `i8`, `i16` or `i32`,
  and `i1` for `c_bool`, with `signext` for a signed type narrower than 32
  bits and `zeroext` for an unsigned one or `c_bool`. The count of
  registers of ADR-0052 decides which arguments go on the stack.
- **Results of a call.** A result takes its C type, and `c_bool` a byte,
  with no extension attribute; TyPy extends or tests the value itself.
- **Exports.** A wrapper takes and returns a `bool` as `i1`, marked
  `zeroext` where clang marks it: on x86-64 and on Apple's arm64.
- **Variadic C functions** stay unsupported until FFI-10 in 1.0.

## Consequences

- On Linux, calls of C functions do not change.
- A C function reads the values it reads under CPython on every target,
  including one whose type is wider than its declaration, since registers
  carry `libffi`'s words and Apple's stack its bytes.
- An export reads a `bool` correctly from a caller on AArch64 Linux.
- A test calls C functions with more narrow arguments than registers, so
  some go on the stack, on each target.

## Related requirements

FFI-01, FFI-02, FFI-05, FFI-12, FFI-13, ADR-0044, ADR-0047, ADR-0052,
ADR-0055.
