# ADR-0107: Read list items and divide inline values in place

## Status

Accepted

## Date

2026-11-23

## Context

ADR-0102 put arithmetic on inline ints in place and brought the benchmark
suite from 7.62 to 5.33 times the time of the C programs, against the 5 of
NFR-22. On 2026-11-23 the suite took 5.13 times as long, and two workloads
held most of the distance: spectralnorm at 22.5 and fannkuchredux at 18.9.

Both spend their time on operations that may raise, so ADR-0102 left them
to the runtime. Every `list[i]` and `list[i] = x` called `typy_rt_list_get`
or `typy_rt_list_set`, which can raise `IndexError`. spectralnorm's inner
function called `typy_rt_int_floor_divide` for `// 2`, which can raise
`ZeroDivisionError`, `typy_rt_int_to_float`, which can raise
`OverflowError` for a large integer, and `typy_rt_float_divide` for
`1.0 / x`. A call that may raise also returns its exception beside its
result, so the optimiser cannot fold it into the loop around it.

A list is a `#[repr(C)]` `ListObject`: its length at offset 8, its item
kind at 24 and the address of its items at 32. Items of the kinds `int`,
`float`, `bool`, an object and `None` take one word each in their slots.

## Options considered

1. **Write the case that cannot raise in place, and call the runtime for
   the rest.** A read tests that the index is an inline int inside the list,
   counting from the end for a negative one, and that the items are words,
   then loads the slot. A write also needs the item and the one it replaces
   to need no reference count: a `float`, a `bool`, or two inline ints.
   `//` and `%` of inline words compute Python's floored result from LLVM's
   truncating division where the divisor is not zero, `float()` of an
   inline word is LLVM's `sitofp`, and `/` of floats divides where the
   divisor is not zero. Every other case calls the runtime as before.
2. **Link the runtime as LLVM bitcode and inline it.** The experiment X10
   of the command line research note measured 4.96 times C against 5.29
   for the archive, but each build took 1.95 to 3.7 seconds instead of
   about 0.1, against the build-time budgets of NFR-04 and NFR-05. ADR-0104
   keeps it as an opt-in, CLI-31, planned for 0.13.
3. **Prove indices in range and divisors non-zero at compile time.** A
   range analysis would drop the tests altogether, but it is a larger
   change than the measured cost asks for, and the in-place tests cost a
   few instructions.

## Decision

Option 1. `typy-codegen` emits the in-place path and the runtime call for
`ListGet`, `ListSet`, `ListSetMoved`, `IntFloorDivide`, `IntModulo`,
`IntToFloat` and `FloatDivide`, and joins their results and exceptions, so
the code after them is unchanged. `llvm.expect` lays the in-place path out
first. The one quotient of inline words outside the inline range,
`-2**62 // -1`, goes to the runtime. A test of the runtime pins the list
layout and the item kind codes that the generated code reads.

The element address is computed from the integer the items pointer holds,
because inkwell builds an element address only in unsafe code, which every
compiler crate forbids.

## Consequences

- The suite's geometric mean against C falls from 5.13 to 3.91, inside
  NFR-22: spectralnorm from 22.5 to 10.2, fannkuchredux from 18.9 to 5.2,
  nbody from 4.8 to 3.8, fasta from 5.2 to 3.9 and dispatch from 4.8 to 3.7
  (2026-11-23, release builds, median of three runs on one core).
- Results do not change: a test program prints `//`, `%` and `float()`
  across the inline range and past it, `/` of every pair of special floats,
  and list reads and writes inside and outside a list, and matches
  CPython 3.15 in debug and release builds; conformance programs for EXP-02
  and DAT-11 run them on every oracle.
- A change to `ListObject` or to the item kind codes fails the runtime's
  layout test until the generated code follows it.
- Lists of `int | None`, of tuples and of unions, and writes that count a
  reference, keep calling the runtime.

## Related requirements

NFR-22, EXP-02, EXP-03, DAT-11, MEM-02, ADR-0102, ADR-0104.
