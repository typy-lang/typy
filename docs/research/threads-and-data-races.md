# TyPy research: threads and data races

| Field | Value |
| --- | --- |
| Document | TyPy research: threads and data races |
| Version | 0.1 |
| Status | Draft |
| Owner | typy-lang contributors |
| Last updated | 2026-11-21 |

## 1. Purpose

This document answers the research OI-31 asks for: which concurrency model
TyPy takes, and whether safe code promises freedom from data races. The
answer fixes the object header and the way reference counts change, which
the stable ABI of 1.0 freezes (ADR-0074, MOD-16), so it is settled before
1.0 although threads come after it (SRS section 6.2).

It builds on the software requirements specification, version 1.115, on
TyPy's reference counting (ADR-0001, MEM-01), its per-thread stacks for
libraries (ADR-0075, FFI-15), and its aims: Python's simplicity, native
speed, and memory and type safety (NFR-07).

## 2. Method

The sources in section 11 were read on 2026-11-21. Three experiments ran
in the development container the same day, on an 8-core x86-64 machine:

- **X6** builds TyPy 0.10.0's runtime twice from the source of tag
  `v0.10.0`: as released, and with `typy_rt_increment` and
  `typy_rt_decrement` changing the count with atomic instructions, an
  increment with relaxed order and a decrement with release order and an
  acquire fence before an object is freed, as a runtime shared by threads
  must. It builds the twelve programs of `tests/bench/programs` with each
  runtime and `--release`, runs each five times on one pinned core with
  the sizes `bench-compare` uses, and compares the medians. Only the two
  functions change; other code that writes a count directly stays plain,
  so the cost measured is a lower bound.
- **X6b** builds a third runtime whose two functions add the test biased
  counting makes on every change: read the running thread's ID, compare it
  with a word of the object's header, and take the atomic path only for an
  object another thread owns. The program runs on one thread, so the
  plain path always runs, and the measure is the test's own cost. The
  plain runtime is timed again beside it.
- **X9** asks each CPython oracle whether it is a free-threaded build and
  whether it has `concurrent.interpreters`.

## 3. What threads mean in Python

A Python program that starts threads shares every object between them.
Under CPython's global interpreter lock one thread runs Python code at a
time, so a single operation on a list or a dict is atomic, though a
sequence of operations is not. CPython now offers two other models:

| Model | Since | What is shared | How objects stay valid |
| --- | --- | --- | --- |
| The global interpreter lock | The default build of every version | Every object | One thread runs at a time |
| Free threads (PEP 703) | 3.13 optional, 3.14 supported (PEP 779) | Every object | Biased reference counts, immortal objects, deferred counts, a lock in each list, dict and set |
| Isolated interpreters (PEP 734) | 3.14, `concurrent.interpreters` | Nothing but queues and buffers | Each interpreter owns its objects; an object sent through a queue is copied, by pickling |

X9 found that the oracles CPython 3.12.16 to 3.15.2 TyPy tests against are
built with the global interpreter lock, and that 3.14 and 3.15 provide
`concurrent.interpreters`.

The cost of free threads in CPython: PEP 703 measured 5% to 8% on one
thread with pyperformance, the largest part from biased reference
counting and the next from the locks; the 3.14 documentation reports about
1% on macOS AArch64 and 8% on x86-64 Linux; PEP 779 sets 15% of speed and
20% of memory as the limits for the supported build.

## 4. How other languages keep threads safe

- **Rust** compiles a program only when every value shared between
  threads is `Sync` and every value moved to another thread is `Send`,
  so safe Rust has no data races. A shared count is `Arc`, with atomic
  counts; a count kept in one thread is `Rc`, with plain ones.
- **Swift 6** checks data races when it compiles: a value crosses between
  isolation domains, such as two actors, only when it is `Sendable`
  (SE-0302, Swift 5.7), or when the compiler proves no other reference to
  its region is left (SE-0414, Swift 6.0).
- **Java and Go** share every object, define what a racing program may see
  (Java's memory model), and keep memory safe whatever the program does,
  with a collector instead of counts; a race gives wrong values, never a
  wrong layout.
- **CPython's free threads** take Java's way with counts: every object may
  be shared, every count change is safe, and the locks inside the
  containers keep each operation whole.

## 5. X6: what atomic counts cost TyPy

Each time is the median of five runs. Every program printed the same
output with each runtime.

| Program | Plain | Atomic | Change | Plain, again | Owner test | Change |
| --- | --- | --- | --- | --- | --- | --- |
| nbody | 2.876 s | 3.221 s | +12.0% | 2.844 s | 2.775 s | -2.4% |
| spectralnorm | 11.895 s | 12.188 s | +2.5% | 11.448 s | 11.280 s | -1.5% |
| fannkuchredux | 7.023 s | 7.142 s | +1.7% | 6.704 s | 7.026 s | +4.8% |
| binarytrees | 1.421 s | 1.651 s | +16.2% | 1.345 s | 1.359 s | +1.0% |
| mandelbrot | 0.852 s | 0.865 s | +1.6% | 0.827 s | 0.804 s | -2.9% |
| fasta | 3.321 s | 3.309 s | -0.4% | 3.264 s | 3.269 s | +0.2% |
| knucleotide | 3.059 s | 3.081 s | +0.7% | 2.963 s | 2.981 s | +0.6% |
| strings | 2.711 s | 2.763 s | +1.9% | 2.741 s | 2.679 s | -2.2% |
| dicts | 2.279 s | 2.269 s | -0.4% | 2.262 s | 2.253 s | -0.4% |
| dispatch | 1.716 s | 1.957 s | +14.0% | 1.711 s | 1.718 s | +0.4% |
| recursion | 0.783 s | 0.805 s | +2.8% | 0.780 s | 0.815 s | +4.4% |
| parser | 2.641 s | 2.699 s | +2.2% | 2.639 s | 2.630 s | -0.3% |
| Geometric mean | | | +4.4% | | | +0.1% |

The plain runtime, timed twice, differs by up to 5.7% between the two
runs (`binarytrees`), so a change under about 5% is within the noise of
this machine.

- Atomic counts cost 4.4% over the twelve programs, and 12% to 16% on the
  three that change counts most: `binarytrees`, which allocates and frees
  trees, `dispatch`, which calls methods through objects, and `nbody`,
  whose bodies are objects. They would cost more on a machine where two
  threads touch the same counts.
- The owner test of biased counting costs nothing measurable: 0.1% over
  the twelve programs, each within the noise.

So biased counting keeps a single thread at the speed of plain counts,
as PEP 703 found for CPython, and only objects that another thread
touches pay for atomic counts. What X6 does not measure is the lock each
list, dict and set needs under free threads, which PEP 703 names as the
second cost after the counts.

## 6. Options

### 6.1 Option W: isolated workers

Each thread runs its own heap, as PEP 734's interpreters do. A value
passes between workers by a copy through a queue, and immutable values,
such as a `str`, a tuple of them or a frozen dataclass, may be shared as
immortal or atomically counted objects. Counts inside a worker stay plain.

### 6.2 Option F: free threads

Every object may be shared. Counts change atomically, or with biased
counts: plain in the thread that owns the object and atomic elsewhere.
Each list, dict and set takes a lock for an operation that changes it, as
CPython's free-threaded build does. Programs run as under CPython's
`threading` module.

### 6.3 Option S: sharing the checker proves safe

The checker decides which values may cross between threads, as Swift 6
and Rust do: a value whose type is immutable, or one the sending thread no
longer reaches, passes without a copy; a mutable value shared by two
threads is refused unless it is a lock or a type built for sharing. Counts
change atomically only for the objects the checker sees shared.

## 7. Comparison

| Measure | W: isolated workers | F: free threads | S: checked sharing |
| --- | --- | --- | --- |
| Runs CPython's `threading` programs | Only those that share nothing mutable | Yes | Those the checker proves |
| Runs `concurrent.interpreters` programs | Yes | Yes | Yes |
| Speed of one thread | Unchanged | Unchanged with biased counts (X6b), plus the container locks | Unchanged for values not shared |
| Memory safety | Yes | Yes, with locks in containers | Yes |
| Freedom from data races in safe code | Yes | No: races give CPython's results, never a bad layout | Yes |
| Object header | Unchanged | An owner field and a shared count for biased counts | A flag for objects that may be shared |
| Work | A heap per thread and copying queues | Atomic or biased counts, container locks, a thread-safe allocator | A sharing analysis in the checker, plus W or F underneath |

## 8. Findings

1. CPython offers both a shared-everything model (free threads, supported
   since 3.14) and an isolated one (interpreters, since 3.14), so either is
   standard Python.
2. A race-free promise for safe code is possible only by isolating
   threads (W) or by checking what is shared (S).
3. Atomic counts cost TyPy 4.4% on average and up to 16% on one thread;
   biased counts cost nothing measurable there (X6, X6b).
4. Biased counts need an owner and a second count in each object's
   header, which the stable ABI of 1.0 fixes; the header word it needs is
   the one decision that cannot wait for threads themselves.

## 9. Recommendation

1. **Free threads (F), with biased counts,** for `threading`, so programs
   written for CPython's threads run unchanged. Safe code is memory safe
   under any interleaving, as in Java and in CPython's free-threaded
   build; it does not promise freedom from data races, which no shared-
   everything Python can.
2. **Isolated interpreters (W)** through `concurrent.interpreters`, for
   programs that want isolation, as CPython 3.14 offers it.
3. **Race diagnostics (S) as warnings,** where the checker proves that two
   threads change one object without a lock, so `--deny-warnings` gives a
   project the stricter promise without rejecting valid Python.
4. **Before 1.0,** the object header takes the owner word and the shared
   count of biased counting, so the stable ABI does not change when
   threads come; the container locks are measured when they are built.

## 10. Questions for the requirements

| ID | Question |
| --- | --- |
| OI-31 | Which concurrency model does TyPy take, and does safe code promise freedom from data races? |

## 11. References

| Source | Version or date | Location |
| --- | --- | --- |
| PEP 703, making the global interpreter lock optional | Python 3.13 | <https://peps.python.org/pep-0703/> |
| PEP 779, criteria for supported free threads | Python 3.14 | <https://peps.python.org/pep-0779/> |
| PEP 734, multiple interpreters in the standard library | Python 3.14 | <https://peps.python.org/pep-0734/> |
| Python support for free threading | Python 3.14 documentation, read 2026-11-21 | <https://docs.python.org/3/howto/free-threading-python.html> |
| SE-0302, `Sendable` and `@Sendable` closures | Swift 5.7 | <https://github.com/swiftlang/swift-evolution/blob/main/proposals/0302-concurrent-value-and-concurrent-closures.md> |
| SE-0414, region based isolation | Swift 6.0 | <https://github.com/swiftlang/swift-evolution/blob/main/proposals/0414-region-based-isolation.md> |
| The Rust reference, `Send` and `Sync` | Rust 1.0 and later | <https://doc.rust-lang.org/nomicon/send-and-sync.html> |
