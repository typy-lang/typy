# ADR-0063: Compile a generator as a resumable function with its frame in its object

## Status

Accepted

## Date

2026-11-19

## Context

0.9 adds generators (SMT-11, FUN-07). Calling a generator function runs
none of its body: it gives a generator object, and each `next()`,
`send()` or `throw()` runs the body from where it last stopped to its
next `yield`, so the body's variables outlive each step. A compiled TyPy
function keeps its variables in its machine stack frame, which is gone
once it returns, and an object the compiler proves does not escape lives
in that frame too (MEM-03, ADR-0042).

CPython 3.12.16 and 3.15.2 behave alike on these points, checked on
2026-11-19. The type of a generator is `generator`, and its `repr` is
`<generator object outer.<locals>.inner at 0x...>`. `next()` on a
generator that is running raises `ValueError: generator already
executing`; a `StopIteration` that escapes the body becomes
`RuntimeError: generator raised StopIteration`; `next()` on a finished
generator raises `StopIteration` with no arguments. `send()` of a value
other than `None` to a generator not started raises `TypeError: can't
send non-None value to a just-started generator`, and `throw()` before
the start raises the exception and finishes the generator. A `return
value` ends the generator with `StopIteration(value)`, which is also the
value of `yield from` over it.

`typing` and `collections.abc` keep three types apart: `Iterable[Y]`
offers `iter()`, `Iterator[Y]` adds `next()`, and `Generator[Y, S, R]`
adds `send()` of an `S`, `throw()`, `close()` and the result `R`.
`collections.abc.Generator` derives from `Iterator`, which derives from
`Iterable`, and `Iterator` has none of `send`, `throw` and `close`.

## Options considered

1. **A resumable function whose frame lives in the generator object.**
   The compiler turns the body into one function that takes the generator
   object and a resumption, jumps to the point after the `yield` it last
   stopped at, and keeps every variable of the body in the object between
   steps.
2. **A machine stack for each generator,** switched to at each step. The
   body is compiled as it is, but each generator needs a stack of its own,
   at least a few pages; switching stacks needs code of its own on each
   target; and each stack needs the stack check of FUN-02.
3. **LLVM's coroutine intrinsics,** which split a function at its
   suspension points and lay out the frame. LLVM then decides what the
   frame keeps, after TyIR's ownership and escape passes have run, so a
   value TyIR borrows across a suspension would be kept without its
   reference, and every pass of TyIR would have to know the intrinsics.

## Decision

Option 1.

- **The object.** A generator object is the header with the type ID 27,
  a descriptor pointer, its state, then its frame. The descriptor holds
  the name, the `__qualname__`, the resume code and the layout of the
  frame. The state is one of not started, suspended at the yield of a
  given number, running and finished. The frame holds the arguments, the
  closure of a nested generator function, then every variable and hidden
  slot of the body, laid out by decreasing alignment (MEM-11).
- **A call** of a generator function allocates the object, takes a
  reference to each argument, and runs nothing of the body.
- **A step** calls the resume code with the object and the resumption:
  next, send with a value, or throw with an exception. It returns that
  the body yielded, with the value; that it returned, with the result;
  or that it raised. The resume code's slots are fields of the frame.
- **A `yield` is a statement:** `yield value`, `target = yield value`,
  `yield from iterable` and `target = yield from iterable` (SMT-11). The
  value sent arrives in the target after the step resumes, so no value
  of TyIR is live across a yield, which the TyIR verifier checks. A
  `yield` inside a larger expression is planned for 1.0 (FUN-12).
- **Nothing is borrowed across a yield.** The object owns the arguments,
  and a generator places no object in its machine frame (MEM-03), since
  that frame is gone between steps.
- **The types are CPython's.** A generator function is annotated
  `Generator[Y, S, R]`, `Iterator[Y]` or `Iterable[Y]` (FUN-07), and its
  call gives a value of that type. `Generator[Y, S, R]` fits where an
  `Iterator[Y]` or an `Iterable[Y]` is expected, never the reverse. Only
  `Generator` offers `send()`, `throw()` and `close()`; with `Iterator`
  or `Iterable`, `S` and `R` are `None`, so a `return value` is an error,
  as type checkers report it.
- **The errors are CPython's:** re-entering a running generator, a
  `StopIteration` that escapes the body, a step of a finished generator,
  and a value sent to a generator not started.
- **Iteration** reaches a generator through the next entry of its type
  (ADR-0065), so `for`, `next()`, `list()`, `sum()` and every builtin
  that takes an iterable takes a generator.

## Consequences

- A generator is one allocation, whatever its body does (NFR-20), and a
  program of many generators needs no stack for each.
- The same code runs on every target, since a step is an ordinary call.
- A variable of a generator is read through the object, not from a
  register kept across steps; within a step LLVM keeps values in
  registers as for any function.
- A chain of `yield from` uses one machine frame for each generator in
  it while a step runs, as CPython's frames do, so the stack check of
  FUN-02 raises `RecursionError` for one too deep.
- A `yield` inside a larger expression, such as `print((yield value))`,
  is E0201 until 1.0.

## Related requirements

SMT-11, FUN-07, FUN-12, NFR-20, MEM-03, MEM-11, FUN-02, ADR-0014,
ADR-0042, ADR-0064, ADR-0065.
