# ADR-0072: Delegate `yield from` one step at a time through the runtime

## Status

Accepted

## Date

2026-11-19

## Context

`yield from source` makes a generator yield each item the iterator of
`source` yields and gives what a generator source returned (SMT-11,
FUN-07, ADR-0063). PEP 380 defines how it passes on what resumes the
generator: a value `send` gives is sent on, `None` and `next()` are
`next()`, an exception `throw` gives is thrown on, and `GeneratorExit`
closes the source first and then leaves. CPython 3.12.16 and 3.15.2,
checked on 2026-11-19, agree on these, raise `AttributeError` for a value
sent on to an iterator that is no generator, and end a generator with
`StopIteration(value)` for `return value` and with a `StopIteration` of no
arguments for `return None`. Their message for the `AttributeError` names
the iterator's type, `list_iterator` in 3.12, `tuple` in 3.15 for a list
display it compiles as a tuple.

A value a generator is sent arrives as the bits of its `S`, so a
generator can pass those bits on only to one whose `S` holds them alike.

## Options considered

1. **Compile the whole protocol into the resume function,** with a test
   of the source's kind and a branch for each way a step can resume.
2. **One runtime call for each step of the delegation,** which reads how
   the delegating generator was resumed from its object and resumes the
   source the same way; the compiled loop only yields the item and
   resumes.

## Decision

Option 2.

- **The loop.** The source's iterator waits in a hidden slot, closed on
  every way out as a loop's iterator is (ADR-0069).
  `typy_rt_generator_delegate(generator, iterator, first)` takes each
  step and answers whether the source yielded; the generator then yields
  that item, converted to its own item type, and the next step passes on
  what resumed it. When the source ends, `target = yield from source`
  assigns what it returned.
- **The result.** `return value` keeps the value in the generator as a
  one-item tuple, through `typy_rt_generator_set_result`, unless it is
  `None`. `next()`, `send` and `throw` raise `StopIteration` with it as
  the argument, and a loop drops it. `yield from` reads it from the
  source; a source that holds none gives `None`, or, for an `R` that
  cannot be `None`, raises `TypeError`, which only a generator that ended
  before the `yield from` reached it can cause.
- **The types.** The items of the source must fit the generator's `Y`. A
  generator source's `S` must hold what the generator is sent: the same
  type, or an optional of it, or the reverse, but never an optional tuple
  for a tuple, which is held otherwise. The value of `yield from` is the
  source's `R`, and `None` for any other iterable.
- **The message** for a value sent on to an iterator that is no generator
  is CPython 3.12's, with the iterator's type name.

## Consequences

- Every resumption of a delegating generator is one runtime call more
  than a plain `yield`, whatever the source is.
- A chain of `yield from` resumes each generator of the chain in turn,
  one machine frame each, as ADR-0063 expected.
- A `yield from` of a generator that had already ended raises
  `TypeError` where its `R` cannot be `None`, where CPython gives `None`
  and a type checker accepts the program; no other program sees the
  difference.

## Related requirements

SMT-11, FUN-07, FUN-11, ADR-0063, ADR-0067, ADR-0069.
