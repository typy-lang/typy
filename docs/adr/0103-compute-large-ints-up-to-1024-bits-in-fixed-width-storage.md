# ADR-0103: Compute large ints up to 1024 bits in fixed-width storage

## Status

Accepted

## Date

2026-11-22

## Context

An `int` outside the inline range is a large integer: an object with a
sign, a length and that many 64-bit limbs. Every operation on one copied
each operand's limbs into a new array from `malloc`, which grew one
`realloc` at a time, computed into another such array and copied the result
into a new object. On 2026-11-22 a loop of addition, multiplication, floor
division, modulo and comparison on values of a given width took, against
CPython 3.15:

| Bits | 64 | 128 | 256 | 512 | 1024 | 2048 | 4096 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TyPy against CPython | 2.75 | 2.74 | 3.77 | 5.03 | 5.28 | 4.60 | 4.05 |

Most programs that leave the inline range stay within a few hundred bits:
hashes, checksums, 128-bit identifiers, fixed-point arithmetic and
cryptographic sizes of 256 to 1024 bits.

## Options considered

1. **Size classes up to 1024 bits.** Values of 64, 128, 256, 512 and 1024
   bits, 1 to 16 limbs, are computed in fixed-width storage that needs no
   `malloc`, and only longer values use a growing heap array. Objects are
   allocated in the same classes.
2. **Size classes up to 512 bits,** one AVX-512 register. Smaller storage,
   but values of 1024 bits, and the product of two 512-bit values, take the
   heap path.
3. **SIMD arithmetic for the classes.** A release build targets the
   baseline CPU of its architecture (NFR-06), so it cannot assume AVX-512,
   and additions and multiplications carry from limb to limb, which vector
   instructions do not speed up; only the bitwise operations and
   comparisons would gain.

## Decision

Option 1, chosen by the project on 2026-11-22 with 1024 bits as the
largest class. `int::limbs::Limbs` holds up to `INLINE_LIMBS`, 16 limbs,
in place, uninitialised until written, and moves to `malloc` only when it
grows past them, so an operation on values up to 1024 bits allocates
nothing but its result object. Arithmetic stays scalar, with carries in
128-bit integers that the compiler turns into add-with-carry chains.
Allocating result objects in the classes, computing on the operands' limbs
without copying them, and floor division without the software 128-bit
division come next, each measured against CPython before it lands.

## Consequences

- The same loop now takes 1.92, 1.84, 1.80, 1.73, 2.11, 2.33 and 2.16
  times CPython's time at 64 to 4096 bits: an operation no longer makes or
  grows heap arrays, and products and quotients of wider values gain as
  their intermediates fit.
- A `Limbs` value is 152 bytes, held on the stack, and moving one copies
  them; the copies of operands and results are what remains of the gap to
  CPython.
- Results do not change: a test program prints every operation on values
  up to 2^1024 and matches CPython 3.15 in debug and release builds.

## Related requirements

NFR-21, NFR-22, DAT-01, EXP-02, ADR-0102.
