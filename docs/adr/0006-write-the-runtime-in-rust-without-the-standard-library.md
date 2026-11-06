# ADR-0006: Write the runtime in Rust without the standard library

## Status

Accepted

## Date

2026-11-06

## Context

Every TyPy binary links a runtime library. It holds what compiled code
cannot work without: program entry, the allocator hooks, reference counting
and the cycle collector (ADR-0001), exception objects and traceback printing
(ADR-0004), unlimited-size integers (DAT-01), `float` formatting that
matches CPython (DAT-02), and the primitives for `str`, `list`, `dict` and
`set`.

The runtime shall be smaller than 48 KB, and smaller than 32 KB without its
large-integer and cycle-collection parts (NFR-02). It is code that every
program trusts, so a defect in it breaks the memory safety promise of
NFR-07 for every program. The pack proposed C, or TyPy code inside
`unsafe` blocks.

## Options considered

1. **C.** The smallest output and the simplest link. But C gives no memory
   safety to the most delicate code in the product, the collector, the
   large-integer arithmetic and the hash tables. It also needs a second
   toolchain, test framework and coverage tool.
2. **Rust with its standard library.** Memory safety and a rich library. But
   `std` brings in formatting, panicking with unwinding, and thread support,
   which do not fit in 32 KB.
3. **Rust without the standard library (`no_std`), and without the `alloc`
   crate.** Memory safety for the runtime's own logic, `unsafe` confined to
   the few places that touch raw memory, one toolchain and one coverage tool
   for the whole project. Size has to be watched: `core::fmt` and panic
   messages are large.
4. **TyPy itself.** The runtime would be written in the language it
   supports. But TyPy has no way to express raw memory access (ADR-0003),
   and the compiler does not exist yet.

## Decision

The runtime is the crate `typy-runtime`, written in Rust with `#![no_std]`,
without the `alloc` crate, and built with `panic = "abort"`. It is compiled
for each target into a static archive, `libtypy_runtime.a`, that `typy`
passes to the linker. It calls only the C library, through a small set of
declared functions: `malloc`, `realloc`, `free`, `write`, `read` and
`abort`, and the `libm` functions that `math` needs.

The release build of the runtime does not use `core::fmt`. Integer and
`float` formatting are written in the runtime, which DAT-02 needs anyway.

## Consequences

- The runtime's own logic is memory safe, and every `unsafe` block in it is
  small, commented with the reason it is sound, and covered by tests under
  AddressSanitizer and Miri.
- The project keeps one language, one build tool and one coverage report.
- The archive's size is checked in CI for both NFR-02 figures. A change
  that pulls in `core::fmt` or panic strings shows up as a size failure.
- Unused runtime functions are removed by the linker's section garbage
  collection, so a program pays only for the parts it uses.
- Rust bundles its own LLVM version, which can differ from the LLVM 23 that
  `typy-codegen` uses (ADR-0005). Link-time optimisation across the runtime
  and the program is therefore not planned. The runtime's hot paths are
  small enough to be written with that in mind.
- The runtime and the compiler are built together from one source tree, and
  `typy` refuses at start-up a runtime archive whose version does not match
  its own.

## Related requirements

NFR-02, NFR-07, NFR-08, NFR-11, DAT-01, DAT-02, MEM-01 to MEM-08, EXC-01,
ADR-0001, ADR-0003, ADR-0004, ADR-0005.
