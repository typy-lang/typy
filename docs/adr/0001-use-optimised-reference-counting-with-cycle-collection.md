# ADR-0001: Use optimised reference counting with selective cycle collection

## Status

Accepted

## Date

2026-11-05

## Context

TyPy compiles standard Python to native code, and a compiled program shall
run alike under TyPy and CPython (STD-02). CPython frees an object as soon as
its last reference goes, and it collects reference cycles with a separate
collector. Programs observe when objects are freed: a file closes, and the
memory returns, at a point the reader of the code can name.

The requirements set the limits for any scheme:

- No tracing garbage collector and no pauses (MEM-01).
- A runtime library under 32 KB (NFR-02, with OI-10 open).
- No new syntax or names, so lifetimes cannot be annotated (STD-07).
- Objects are shared freely: two names can refer to one list, and a change
  through one is seen through the other (DAT-07).
- Memory use is a goal of its own: an object header of at most 8 bytes
  (NFR-12), and a `list[int]` of one million values within 8.5 MB (NFR-13).
- Programs are single-threaded up to 1.0 (A-03).

## Options considered

1. **Tracing garbage collector**, as in Java, Go and PyPy. It handles cycles
   with no extra work. But it pauses, it adds a large runtime, and it frees
   objects later than CPython does, so a program that relies on prompt
   cleanup behaves differently. It fails MEM-01 and NFR-02.
2. **Ownership and borrowing checked by the compiler**, as in Rust. It needs
   no counts at run time. But it needs lifetime annotations or new syntax,
   which STD-07 forbids, and it cannot express Python's free sharing
   (DAT-07).
3. **Arenas or regions.** Allocation and freeing are cheap. But Python has no
   way to say how long an object lives, so the compiler cannot pick a region
   for most objects.
4. **Plain reference counting**, with a count update on every copy of a
   reference. It frees at the same points as CPython and is simple. But every
   assignment and every argument touches memory, which puts NFR-03 out of
   reach.
5. **Reference counting that leaks cycles**, as the specification pack
   proposed. It is small and fast. But a long-running program with parent
   links or graphs grows without bound.
6. **Reference counting optimised when the program is compiled, with cycle
   collection only for types that can form a cycle.** Swift, Lean 4 and Nim 2
   (its ORC mode) use variants of this. It frees at the same points as
   CPython, removes most count updates, and charges for cycle collection only
   where a cycle is possible.

## Decision

TyPy uses option 6, in six layers. Each layer applies where the compiler can
prove it safe, and otherwise the next layer handles the object.

1. **Values without an object.** An `int` that fits in 63 bits, a `float`, a
   `bool` and `None` are held directly in registers, variables, fields and
   containers, with no heap allocation. A larger `int` moves to the heap
   (DAT-01). A small tuple is held as a plain structure, because `is` is
   not accepted on tuples (EXP-08), so copying it cannot be observed.
2. **Stack allocation.** An object that escape analysis proves does not
   outlive its function is allocated on the stack (MEM-03).
3. **Count updates removed when compiling.** Parameters are borrowed and cost
   no count update. The last use of a variable moves its value. Memory freed
   by one object is reused for a new object of the same size, as in the
   Perceus technique (MEM-04, MEM-05).
4. **Counts at run time.** Each heap object has an 8-byte header: a 32-bit
   count and a 32-bit type ID (NFR-12). Counts are not atomic while programs
   are single-threaded. An object is freed when its count reaches zero
   (MEM-01).
5. **Cycle collection for some types only.** The compiler reads the class
   definitions and finds every type whose references could lead back to an
   object of the same type. Only instances of those types are tracked. A
   synchronous trial-deletion collector, after Bacon and Rajan, frees their
   cycles at points set by an allocation count, so collection is
   deterministic and needs no thread (MEM-06). The collector is linked only
   when a program has such a type.
6. **Allocator.** Small objects come from size-class free lists, and large
   objects from `malloc`. A project can replace the allocator with its own
   allocate, reallocate and free functions (MEM-08).

Two layouts follow from this decision:

- A string stores each code point in 1, 2 or 4 bytes, chosen when the
  string is created, as CPython does under PEP 393. Indexing and `len()`
  count code points in constant time (DAT-05).
- A container's layout depends on its element type. `list[int]` holds
  63-bit tagged words, `list[float]` holds raw doubles, and `list[Point]`
  holds pointers (MEM-02, NFR-13).

## Consequences

- Objects are freed at the same points as under CPython, for every object
  that is not part of a cycle. Programs that close files or release memory
  by dropping the last reference behave the same under both.
- There are no pauses, and the run-time cost of memory management is
  spread evenly across the program.
- Most programs pay nothing for cycles, because `str`, `list[int]`,
  `dict[str, int]` and most classes cannot form one.
- Storing into a field or a list item still updates two counts. Programs
  that rewire object graphs heavily will be slower than equivalent C, and
  the benchmark suite for NFR-03 shall include such a program.
- The compiler gains three analyses: escape analysis, last-use analysis and
  the type-level cycle analysis. Each needs its own tests, and the memory
  stage shows its result with `--emit=tyir-mem` (CLI-04).
- The cycle collector and the large-integer code add to the runtime size.
  OI-10 tracks the effect on NFR-02.
- Objects in a cycle are freed later than CPython would free them, at the
  next collection point. TyPy does not support `__del__` before 1.0, so a
  program cannot observe this difference.
- Threads, planned for after 1.0, need counts that are safe across threads.
  The path is biased reference counting, which the free-threaded build of
  CPython uses (PEP 703): the owning thread updates a count without atomic
  operations, and other threads use atomic ones. That change needs its own
  decision record.

## Related requirements

MEM-01 to MEM-08, DAT-01, DAT-05, DAT-07, EXP-08, CLI-04, NFR-02, NFR-03,
NFR-07, NFR-08, NFR-12, NFR-13, A-03, OI-08, OI-10.

## References

| Source | Location |
| --- | --- |
| Reinking, Xie, de Moura and Leijen, "Perceus: Garbage Free Reference Counting with Reuse", PLDI 2021 | <https://doi.org/10.1145/3453483.3454032> |
| Ullrich and de Moura, "Counting Immutable Beans: Reference Counting Optimized for Purely Functional Programming", IFL 2019 | <https://arxiv.org/abs/1908.05647> |
| Bacon and Rajan, "Concurrent Cycle Collection in Reference Counted Systems", ECOOP 2001 | <https://doi.org/10.1007/3-540-45337-7_12> |
| Nim 2.0 release, ORC memory management as the default, 2023-08-01 | <https://nim-lang.org/blog/2023/08/01/nim-v20-released.html> |
| PEP 393, flexible string representation | <https://peps.python.org/pep-0393/> |
| PEP 703, making the global interpreter lock optional, biased reference counting | <https://peps.python.org/pep-0703/> |
