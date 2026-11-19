# ADR-0064: Close a generator freed while suspended at its yield

## Status

Accepted

## Date

2026-11-19

## Context

A generator that a program stops using before it finishes still owes its
`finally` clauses and the `__exit__` of its `with` statements, such as
the file a `with open(...)` inside it holds (FUN-11). CPython 3.12.16 and
3.15.2 close such a generator when its last reference goes, checked on
2026-11-19: `break` out of a loop over it runs its `finally` before the
statement after the loop, and so does `del` of the last variable that
holds it. A generator never started runs nothing. `close()` raises
`GeneratorExit` at the suspended `yield`; a generator that catches it and
yields again makes `close()` raise `RuntimeError: generator ignored
GeneratorExit`. `yield from` closes the generator it delegates to first.
An exception raised while a freed generator closes is written to
standard error after `Exception ignored in: <generator object ... at
0x...>`, and the program goes on. When `list.clear()` drops a suspended
generator, its `finally` already sees the list empty.

TyPy frees an object when its last reference goes (MEM-01), when CPython
frees a generator outside a cycle. Until 0.9 freeing an object runs no
code of the program: the runtime releases what it holds, and the
compiler relies on that. An operand evaluated before a call keeps its own
reference only because the call may run code that frees what it borrows
(NFR-07), and the runtime's container operations, such as `list.clear`,
release items while they change the container.

## Options considered

1. **Close the generator when it is freed, as CPython does,** by resuming
   it with `GeneratorExit` at its `yield`.
2. **Release the frame without running the body.** Simple, but a loop
   that leaves a generator early would skip its `finally` and leave the
   file of its `with` open, where CPython closes it.
3. **Refuse `try`, `with` and `finally` around a `yield`,** so closing has
   nothing to run. This rejects common Python.

## Decision

Option 1.

- **`close()`** resumes a suspended generator with `GeneratorExit` at its
  `yield`. When the body ends, by returning or by raising `GeneratorExit`
  or `StopIteration`, `close()` returns `None`; when it yields,
  `close()` raises `RuntimeError: generator ignored GeneratorExit`; any
  other exception leaves `close()`. A generator not started or finished
  is marked finished and runs nothing. A generator suspended in `yield
  from` closes the generator it delegates to first.
- **Freeing** a suspended generator closes it. An exception the closing
  raises is written to standard error as CPython writes it, `Exception
  ignored in: <generator object NAME at 0x...>` with its traceback, and
  the program goes on.
- **Program code runs at a release only in compiled code.** A release in
  compiled code of a value whose type can hold a generator counts as a
  call for the operand-lifetime rule, so an operand evaluated before it
  keeps its own reference (NFR-07). A release inside the runtime, such as
  of an item `list.clear()` drops, puts the suspended generator in a
  queue of the thread, and compiled code closes the generators of the
  queue as soon as that runtime call returns, before the next operation
  of the program.
- **A generator in a reference cycle** is closed when cycle collection
  frees it, in 1.0 (MEM-06); until then it is never freed or closed.

## Consequences

- A loop that leaves a generator early runs its `finally` and closes its
  files where CPython does, and the output comes in CPython's order.
- No runtime operation runs program code while it changes a container:
  a generator it drops is closed once the operation has finished, which
  is also what the program sees under CPython, since CPython's containers
  update themselves before they release items.
- An operand before a release of a value that can hold a generator
  takes a reference of its own, one increment and one decrement more.
- An exception raised while a freed generator closes appears only on
  standard error, with the generator's address, so a conformance test
  does not print one.

## Related requirements

FUN-11, MEM-01, MEM-06, NFR-07, SMT-11, ADR-0063.
