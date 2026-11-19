# ADR-0068: Give the caller back its handled exception when a generator step stops

## Status

Accepted

## Date

2026-11-19

## Context

The runtime keeps one exception being handled, which an `except` clause
sets when it starts and restores when it ends (EXC-04). A new exception
takes it as its context, and a bare `raise` raises it. A compiled
function restores it at the end of each clause, and the value it
restores is the one the clause found at its start.

A generator may `yield` inside an `except` clause or inside a `finally`
block that runs for a pending exception (FUN-07). CPython 3.12.16 and
3.15.2, checked on 2026-11-19, keep the exception a suspended generator
handles with the generator: the caller does not see it while the
generator is suspended, an exception the caller raises then has the
caller's own context, and the generator handles its exception again when
it resumes, so an exception `throw()` raises at the `yield` takes it as
its context. Each step may come from another caller, handling another
exception, so the value a clause found at its start in one step is wrong
for its end in a later step. A resume function also keeps no value of
one step for the next (ADR-0063).

## Options considered

1. **The runtime hands the state over at each step.** A step notes in
   the generator object the exception its caller handles, and restores
   it when the step stops, however it stops. A step that resumes inside a
   clause makes the clause's exception the one being handled again, and
   the outermost clause of the body restores the noted exception when it
   ends.
2. **The compiled code hands it over at each `yield`.** Each `yield`
   inside a clause restores the caller's exception, and each resumption
   sets the clause's again. The caller's exception must still be kept
   somewhere for the outermost clause, and a step that raises out of the
   body needs the same work on its error path.

## Decision

Option 1. `GeneratorObject` has a `caller` field. Each step sets it to
the exception being handled before it calls the resume code, and makes
that exception the one being handled again when the resume code returns.
In a resume function, an `except` clause and a `finally` block run for a
pending exception keep their exception in a hidden slot of the frame,
and no clause keeps the exception handled before it: leaving a clause
restores the exception of the clause around it, or `GeneratorCaller`,
the noted one, for the outermost. Each `yield` inside a clause sets the
innermost clause's exception again where the next step resumes, before
it raises what `throw()` or `close()` gave it.

## Consequences

- An exception raised while a generator is suspended, or raised by the
  generator while its caller handles another, has the context CPython
  gives it.
- A step costs one load and one store more, whatever its body does.
- A function that is not a generator keeps the exception handled before
  a clause in a value of its own, as before.

## Related requirements

FUN-07, SMT-11, EXC-04, EXC-07, ADR-0063.
