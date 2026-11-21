# ADR-0096: Run threads in parallel over shared objects with biased reference counts

## Status

Accepted

## Date

2026-11-21

## Context

OI-31 asks which concurrency model TyPy takes, and whether safe code
promises freedom from data races. The object header and the way reference
counts change are part of the stable ABI of 1.0 (ADR-0074, MOD-16), so the
answer is needed before 1.0, though threads come after it.

The research note "threads and data races" compared three models:
isolated workers, as CPython 3.14's `concurrent.interpreters` (PEP 734);
free threads over shared objects, as CPython's free-threaded build (PEP
703, supported since 3.14 by PEP 779); and sharing that the checker proves
safe, as Swift 6 and Rust do. Its experiment X6 rebuilt TyPy 0.10.0's
runtime: atomic reference counts cost 4.4% over the twelve benchmark
programs on one thread, and 12% to 16% on three of them; the owner test of
biased counting cost 0.1%, within the noise.

TyPy's aims ask for true parallelism, memory safety, the least cost in
speed, and standard Python.

## Options considered

1. **Isolated workers only.** Free of data races and simple, but most
   programs written for `threading` share mutable objects and would be
   rejected.
2. **Sharing the checker proves safe only.** Free of data races, but it
   rejects valid `threading` programs that CPython runs.
3. **Free threads over shared objects,** as CPython's free-threaded build
   runs them.

## Decision

Option 3.

- **Parallelism.** Threads run Python code in parallel over shared
  objects, with no global lock, through `threading`, `queue` and
  `concurrent.futures`, and isolated interpreters through
  `concurrent.interpreters`, the modules CPython provides.
- **Counts.** Reference counts are biased: an object's header holds the
  thread that owns it, a count that only that thread changes with plain
  instructions, and a shared count that other threads change atomically.
  An object is freed when both reach zero. Immortal objects are never
  counted.
- **Containers.** Each list, dict and set takes a lock of its own for an
  operation that changes it, so each operation is whole, as under
  CPython's free-threaded build; reads that find the container unchanged
  take no lock.
- **The promise.** Safe code is memory safe under any interleaving: no
  object is freed while a thread can reach it, and no value is read with
  the wrong layout (NFR-07). Safe code does not promise freedom from data
  races, as neither CPython nor Java does: a race gives one of the results
  CPython's free-threaded build may give.
- **Diagnostics.** The checker may warn where it proves that two threads
  change one object without a lock, so `--deny-warnings` gives a project a
  stricter rule without rejecting valid Python.
- **Before 1.0.** The header takes the owner and the shared count, so the
  stable ABI does not change when threads arrive (MEM-12), and one thread
  pays no more than NFR-27 allows for them.

## Consequences

- Programs written for CPython's threads run unchanged and in parallel.
- The header grows by the owner and the shared count, a cost in memory for
  every object; X6 found no cost in speed on one thread.
- The allocator, the runtime's own state and every container become safe
  for threads, and their locks are measured against NFR-27 when they are
  built.
- A program that depends on the global lock to make a sequence of
  operations atomic needs a `threading.Lock`, as it does under CPython's
  free-threaded build.

## Related requirements

MEM-12, NFR-27, NFR-07, MOD-16, OI-31, ADR-0001, ADR-0074, ADR-0075.
