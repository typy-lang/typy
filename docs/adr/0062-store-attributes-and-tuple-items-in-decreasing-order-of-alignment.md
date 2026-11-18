# ADR-0062: Store attributes and tuple items in decreasing order of alignment

## Status

Accepted

## Date

2026-11-18

## Context

An instance holds the header and its class pointer, 16 bytes, then its
attributes, laid out as an LLVM structure of their types in the order
they are declared (ADR-0012); a tuple is laid out the same way. Each item
takes its own alignment, so a `bool` between two `int`s takes 8 bytes. A
class of an `int`, a `bool`, a `float`, a `bool`, an `int` and a
`Node | None` was laid out on 2026-11-18 as `{ i64, i1, double, i1, i64,
ptr }`: 48 bytes of attributes holding 34 bytes of values, 64 bytes in
all, for which glibc's allocator takes an 80-byte chunk, since a chunk
holds 8 bytes of its own and grows by 16.

The runtime reads attributes and items through a tuple descriptor, which
gives each one's offset, in the order the program declares them (detailed
design section 10.2), so it does not depend on the order in memory.
`repr`, equality, hashing, ordering, `__match_args__`, a dataclass's
fields and `__init__` all go by the declared order.

## Options considered

1. **Declaration order,** as up to 0.7.
2. **Decreasing alignment, in declaration order among items of one
   alignment.** The padding that remains is at the end, and the order
   follows from the types alone, so every build gives the same layout
   (NFR-06).
3. **The fields used most first.** It needs a profile of the program,
   which a build does not have, and would make the layout depend on it.

## Decision

Option 2, for the attributes of an instance of a class the program
defines, those after an exception's fixed part, the items of a tuple, and
the captures of a function object and the value of a cell (ADR-0060,
ADR-0061). A tuple item takes the alignment of its most aligned item and
is laid out by the same rule inside. The descriptor keeps the declared
order, each item with its offset. A `ctypes.Structure` keeps the layout C
gives it (FFI-03).

## Consequences

- The class above takes 40 bytes of attributes, 56 bytes in all, for
  which glibc takes a 64-byte chunk: a fifth less memory for each object,
  and fewer bytes for a loop over many of them to read.
- A program sees the declared order everywhere; only `--emit=llvm-ir` and
  `--emit=asm` show the order in memory.
- The order changes when an attribute's type changes, which reorders only
  the program's own memory, not any interface.
- A descriptor's offset that disagreed with the compiler's layout would
  make the runtime read and free the wrong memory, so a test checks every
  offset of every descriptor against LLVM's offset of the field, and the
  sanitizer run reads and frees each object (NFR-07).

## Related requirements

MEM-11, NFR-18, NFR-06, NFR-07, NFR-12, FFI-03, ADR-0012, ADR-0060, ADR-0061.
