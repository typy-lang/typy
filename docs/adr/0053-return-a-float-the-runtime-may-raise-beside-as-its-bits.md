# ADR-0053: Return a float the runtime may raise beside as its bits

## Status

Accepted

## Date

2026-11-17

## Context

A runtime function that may raise returns its result and the raised
exception together (ADR-0004). The program declares such a function in
LLVM IR as returning a pair, `{double, ptr}` for a `float`, and the runtime
defines it in Rust as returning a `#[repr(C)]` structure of the same two
fields. The two sides are compiled by different front ends, so they agree
only where the target's C convention returns that structure as LLVM
returns the pair.

On x86-64 they agree by chance: both return the `double` in `xmm0` and the
pointer in `rax`. On AArch64 they do not. clang 19, measured on 2026-11-17,
lowers a C function returning `struct {double value; void *raised;}` for
`aarch64-linux-gnu` as returning `[2 x i64]`, in `x0` and `x1`, because a
structure that mixes a `double` and a pointer is not a floating-point
aggregate; the pair `{double, ptr}` that TyPy declares returns in `d0` and
`x0`. Every runtime call that may raise and gives a `float`, such as float
division, the functions of `math`, `round` and `float()` of a string,
would read a wrong value on AArch64. Pairs of an integer, a byte or a
pointer with the raised pointer, such as `{i64, ptr}`, return in `x0` and
`x1` on both sides and agree.

A second, smaller mismatch: the runtime takes a Rust `bool` in a few
functions, such as `typy_rt_cdata_set_bool`, which Rust declares as
`zeroext i1`, while the program passes an `i1` without the attribute. The
C conventions differ on whether the caller or the callee clears the upper
bits of a narrow argument, so the program's declaration must carry the
attributes of the definition, and the caller then extends the value.

## Options considered

1. **Lower the program's runtime calls by the target's C convention.**
   Declare each pair as clang would, `[2 x i64]` on AArch64, and take it
   apart after the call. Correct, but puts a C calling convention in front
   of every internal call and differs between targets.
2. **Return the float's bits.** The runtime returns such a result as the
   64 bits of the `double` in a `u64`, so the pair is `{i64, ptr}`, which
   every 64-bit target returns in two integer registers; the program turns
   the bits back into a `double`, which costs one register move.
3. **Return the float through memory.** The program passes a pointer for
   the value, as for a large result. A store and a load on every call.

## Decision

Option 2.

- Every runtime function that may raise and gives a `float` returns
  `{i64, ptr}`, the value's bits and the raised exception. The runtime's
  `Raising<f64>` becomes a structure over the `u64` bits, and the program
  bitcasts the integer to `double` after the call.
- A runtime function's `bool` parameter is declared `zeroext i1`, and its
  `bool` result `zeroext i1`, as Rust declares them.
- A test of the code generator fails when a runtime declaration returns a
  pair with a floating-point member, or takes or returns an `i1` without
  `zeroext`, so a new runtime function cannot bring either back.

## Consequences

- The program's calls to the runtime have one form on every target, and
  AArch64 needs no lowering of its own for them.
- The change touches every raising float function of the runtime and its
  callers in the code generator, in one step, before AArch64 code is
  generated.
- A future runtime function that returns two floats, or a float and an
  integer, is defined with this rule in mind.

## Related requirements

CLI-08, NFR-16, ADR-0004, ADR-0051.
