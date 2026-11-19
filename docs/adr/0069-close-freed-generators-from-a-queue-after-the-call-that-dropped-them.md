# ADR-0069: Close freed generators from a queue after the call that dropped them

## Status

Accepted

## Date

2026-11-19

## Context

ADR-0064 decided that a generator freed while suspended at its `yield`
is closed, as CPython closes it (FUN-11). It closed a generator that
compiled code frees at once, queued one the runtime frees while it
changes a container, and counted a compiled release as a call for the
operand-lifetime rule (NFR-07). Building it showed three things.

First, the runtime cannot tell a release of compiled code from its own:
compiled code drops a list with `typy_rt_decrement`, and the list's items
go inside that call. Second, a release in compiled code only drops a
temporary that a call, or another operation that is not quiet, made, so
the operands evaluated before it already keep references of their own,
and no extra reference is needed. Third, CPython 3.12.16 and 3.15.2,
checked on 2026-11-19, close the generators of one container in a fixed
order: a list and a tuple drop their last item first, a dict and an
instance's attributes their first; `list.clear()` empties the list
first, then drops its items last first. A loop drops its iterator when it
ends, by `break` or by running out, before its `else` clause, and when an
exception or a `return` leaves its body.

## Options considered

1. **One queue for every freed generator.** Freeing a suspended
   generator never runs code: the generator goes to the end of a queue.
   Compiled code reads the queue after each release and after each
   runtime call that may release an object, and closes what it holds.
2. **Close at once in compiled releases, queue in the runtime,** as
   ADR-0064 has it, with an entry point of the runtime for compiled
   releases. Two paths do the same work, and a generator freed inside a
   runtime call that compiled code made still needs the queue.

## Decision

Option 1.

- **Freeing** a suspended generator puts it in the queue of the thread,
  linked through the object, and frees nothing. A generator not started
  or finished is freed at once and runs nothing.
- **Closing** takes the whole queue, then closes each generator in the
  order it was freed, as `close()` does, and frees it, so one generator
  closes completely, with any generator its own closing drops, before
  the next. An exception the closing raises is written to standard error
  as CPython 3.12 writes it, `Exception ignored in: <generator object
  NAME at 0x...>` with its traceback and last line, and the program goes
  on. A generator that yields again is reported with `RuntimeError:
  generator ignored GeneratorExit` and freed without closing it again.
- **Compiled code** of a program with a generator function tests the
  queue after every release and after every runtime call that takes an
  object other than a `str`, `bytes` or a type, and closes the queue when
  it is not empty. A program without one tests nothing.
- **No release runs code of the program inside the runtime,** so every
  container operation finishes before a generator it dropped closes.
- **Release order is CPython's:** a list and a tuple drop their items
  last first, a dict and an instance first first, and a loop's iterator
  is dropped when the loop ends, before its `else` clause, and on the
  way out of its body by `return` or an exception.

## Consequences

- The queue is empty after every test, so a release inside an
  expression runs code only within an operand that is not quiet, and the
  operand-lifetime rule needs no change.
- A program with generators pays one load and one branch after each
  release and runtime call that may drop an object.
- A loop's iterator no longer lives until its function returns, so the
  memory it holds goes back sooner.
- A function's own variables go in the order CPython 3.12 drops them,
  first first; CPython 3.15 drops them last first, so a test must not
  print the closing of two generators that one function's end drops.

## Related requirements

FUN-11, FUN-07, MEM-01, NFR-07, ADR-0063. Supersedes ADR-0064.
