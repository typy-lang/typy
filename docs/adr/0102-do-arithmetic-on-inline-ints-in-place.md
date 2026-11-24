# ADR-0102: Do arithmetic on inline ints in place

## Status

Accepted

## Date

2026-11-22

## Context

NFR-22 asks that the geometric mean run time of the benchmark suite built
with `--release` be within 5 times that of the C programs by 0.12. On
2026-11-22 TyPy took 7.62 times as long, with spectralnorm at 37,
fannkuchredux at 30 and recursion at 19: the workloads of integer
arithmetic.

An `int` is a 64-bit word: an inline value has its lowest bit set and holds
the number shifted left by one, and any other word is the address of a
large integer. The generated code called the runtime for every `int`
operation, `typy_rt_int_add`, `typy_rt_int_compare` and the others, and for
every reference count of an `int` word. The runtime is linked as an archive,
so the optimiser cannot see into those calls, and a loop counter cost a call
for its addition, one for its comparison and two for its counts.

## Options considered

1. **Write the inline case in the generated code.** Two inline words are
   added as `left + (right - 1)`, which is the inline word of the sum, with
   LLVM's overflow intrinsic: a 64-bit overflow happens exactly when the sum
   leaves the inline range, and then the runtime computes it. Subtraction,
   multiplication, negation, the bitwise operations and the comparisons
   work the same way, and a reference count tests the lowest bit before it
   calls the runtime.
2. **Link the runtime as LLVM bitcode and optimise across it.** The
   runtime is built by Rust's LLVM and the program by TyPy's LLVM 23, whose
   bitcode need not match, and every build would optimise the whole
   runtime, against the build-time budgets of NFR-04 and NFR-05.
3. **Infer fixed-width integers where a value provably fits.** A range
   analysis over every function, which may still need the checked path,
   and a larger change than the measured cost asks for.

## Decision

Option 1. `typy-codegen`'s `small_int` module emits, for `+`, `-`, `*`,
unary `-`, `~`, `&`, `|`, `^` and the six comparisons of two `int` words, a
test of both words' lowest bits, the operation on the words in place, and a
call of the runtime when a word points to a large integer or the result
overflows. Incrementing or decrementing the count of an `int` word calls
the runtime only for a word that points to a large integer. `llvm.expect`
marks the inline case as the likely one, so it is laid out first. Floor
division, modulo, powers and conversions keep calling the runtime.

## Consequences

- The suite's geometric mean against C falls from 7.62 to 5.33: mandelbrot
  from 3.41 to 1.28, recursion from 18.8 to 8.1, spectralnorm from 37 to
  21 and fannkuchredux from 30 to 19 (2026-11-22, release builds, median of
  three runs on one core).
- Each such operation adds a branch and a few instructions to the
  generated code, and the runtime's functions stay for large integers.
- Results do not change: a test program prints every operation on values
  across the inline range and up to 2^1024 and matches CPython 3.15, and a
  conformance program runs the same values on every oracle.

## Related requirements

NFR-22, NFR-03, EXP-02, DAT-01, ADR-0001.
