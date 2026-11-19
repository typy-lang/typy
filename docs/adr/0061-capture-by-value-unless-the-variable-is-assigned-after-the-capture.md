# ADR-0061: Capture by value unless the variable is assigned after the capture

## Status

Accepted

## Date

2026-11-18

## Context

A nested function or a `lambda` reads the variables of the functions
around it when it runs, not when it is made: CPython keeps each captured
variable in a cell, which every function that captures it shares. Three
lambdas made in a loop over `range(3)` all return 2, and `nonlocal`
rebinds the outer variable for every function that sees it, checked with
CPython 3.12 and 3.15 on 2026-11-18. A nested function's defaults are
evaluated when its `def` runs.

TyPy counts references and collects no cycles until 1.0 (MEM-06), so an
object that refers to itself, directly or through others, is never freed,
and the sanitizer run of the conformance suite reports it as a leak
(NFR-07). A nested function that calls itself by name captures the
variable its own `def` binds, and two that call each other capture each
other. An object the compiler proves does not outlive its function lives
in the function's frame (MEM-03, ADR-0042), which is gone once the
function returns, while a closure may outlive it.

## Options considered

1. **Every captured variable in a cell,** as CPython does: an allocation
   for each captured variable, and a read through the cell at every use.
2. **By value when nothing can assign the variable after the closure is
   made, and in a cell otherwise.** A variable that one assignment binds,
   which every `def` and `lambda` that captures it follows, which no loop
   binds again and which no `nonlocal` names, has one value for the whole
   life of the closure, so a copy reads what a cell would.
3. **Always by value.** Wrong for a loop variable and for `nonlocal`.

## Decision

Option 2.

- **A copy** is made when the `def` or `lambda` runs, held among the
  captures of the function object (ADR-0060), with the reference it owns.
- **A cell** is an object of its own: the header with the type ID of a
  cell, then the value as one item of the variable's type, which the
  function and every closure that captures the variable share and the
  runtime releases with the last reference. A captured variable that a
  loop binds, that is assigned after a closure captures it, or that a
  `nonlocal` names lives in a cell for the whole function.
- **A capture is an escape.** An object a closure captures or a cell
  holds is never placed in a frame, so no closure refers to memory its
  function has left (MEM-03).
- **Recursion.** Inside a nested function, its own name, when only its
  `def` binds it, reads the function object that is running, which the
  code takes as its first argument, so it captures nothing and makes no
  cycle.
- **Mutual recursion.** Nested functions of one scope that call one
  another by name, each bound only by its `def`, share one environment of
  the variables any of them captures. Each function object refers to the
  environment, never to the others, and a call by name among them passes
  the environment, so they form no cycle.
- **Defaults** of a nested function and of a `lambda` are evaluated when
  the `def` or `lambda` runs and held among its captures.

## Consequences

- A closure over values that do not change costs one allocation, the
  function object, and its reads are as fast as reads of a local.
- A program sees the same values as under CPython, loop variables and
  `nonlocal` included.
- Recursion and mutual recursion of nested functions free every object,
  which the sanitizer run checks (NFR-07). A nested function used as a
  value by another of its group, or a closure stored in an object it
  captures, still forms a cycle, which cycle collection frees in 1.0
  (MEM-06).

## Related requirements

FUN-04, EXP-14, SMT-09, MEM-03, MEM-06, NFR-07, ADR-0042, ADR-0060.
