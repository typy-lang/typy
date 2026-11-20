# ADR-0073: Bind the arguments of a call with unpacking when the program runs

## Status

Accepted

## Date

2026-11-20

## Context

`f(*values)` passes each value of an iterable as a positional argument,
and `f(**options)` each entry of a dict as a keyword argument (EXP-17).
How many values a list holds, and which keys a dict has, are known only
when the program runs, so the checker cannot bind such a call to the
parameters as it binds every other call (FUN-03). CPython binds it when
the call runs and raises `TypeError` for a call that does not fit.
CPython 3.12.16 and 3.15.2, checked on 2026-11-20, agree on these points:

- A call evaluates its positional arguments, each `*iterable` read to its
  end in turn, before its keyword arguments, even a `*iterable` written
  after a keyword. The keyword arguments and each `**dict` then go into
  one dict in source order, and a name given twice raises `TypeError:
  __main__.sink() got multiple values for keyword argument 'k'`, which
  names the function with its module, before a later keyword argument is
  evaluated.
- Binding then checks the keywords first, in order: one that no
  parameter takes is `got an unexpected keyword argument 'z'`, one for
  positional-only parameters is `got some positional-only arguments
  passed as keyword arguments: 'a, b'`, and one for a parameter a
  positional argument filled is `got multiple values for argument 'b'`.
  Then come too many positional arguments, `takes from 1 to 2 positional
  arguments but 3 were given`, with `(and 1 keyword-only argument)` when
  keywords fill keyword-only parameters, then the missing positional
  arguments, `missing 2 required positional arguments: 'a' and 'b'`, and
  last the missing keyword-only ones. These messages name the function by
  its qualified name, such as `Box.put()` or `outer.<locals>.inner()`.

A type checker accepts `f(*values)` when the item type of `values` fits
each parameter it may reach, and `f(**options)` of a `dict[str, V]` when
`V` fits each parameter a keyword may reach. Values are held unboxed by
type in TyPy, so one runtime list or dict holds values of one type.

## Options considered

1. **Accept only a tuple of a fixed number of items** after `*`, which
   binds as its items would. It leaves out forwarding `*args` and
   `**kwargs` to another function, the main use.
2. **One generic runtime binder** that takes every argument as a word in
   one array and fills the callee's parameters. Values held in different
   ways would need one boxed form, and every call would pay for it.
3. **Bind statically what the checker knows, and the rest in the code of
   the call**, with one runtime routine that checks the names and counts
   as CPython does and raises its `TypeError`.

## Decision

Option 3.

- **Static part.** Positional arguments before the first `*iterable` of
  unknown length, the items of a fixed tuple there, and each keyword
  argument bind as in any call, with E0404 for what can never fit, such
  as a keyword no parameter takes when there is no `**kwargs`.
- **Positional tail.** From the first `*iterable` of unknown length on,
  every positional value goes into one new list, read in source order.
  Its values have one type, that of the parameters it may reach when they
  share one, otherwise the first value's, and each part must hold its
  values alike, E0409. That type must fit every positional parameter from
  the tail's first index on and the items of `*args`, E0404.
- **Keyword dicts.** Each `**` value is a `dict[str, V]` of one `V`. The
  call merges them into one new dict in source order, and raises
  CPython's `got multiple values for keyword argument` as it merges a
  name that an earlier keyword argument or dict gave. `V` must fit every
  parameter a keyword may still reach and the values of `**kwargs`.
- **Check.** `typy_rt_call_check` takes a description of the callee and
  of the call's static part, the number of positional values and the
  merged dict, and raises the `TypeError` CPython raises, in CPython's
  order, or returns.
- **Values.** Each parameter then takes its value: the list's item when
  its index is below the number of positional values, else its keyword
  argument, its entry in the dict, or its default. `*args` takes the
  static extra values and the rest of the list, and `**kwargs` the
  keywords and entries no parameter took, in source order.
- **Scope.** Calls of the program's functions, nested functions, methods,
  classes and `print(*values)`. A `*` or `**` in a call of any other
  built-in, of a function value, or of an exception class, and `**` in a
  call of `print`, are planned for 0.10 (EXP-19).

## Consequences

- A call without unpacking compiles as before and pays nothing.
- A call with unpacking makes a list or a dict, or both, and one runtime
  check, which forwarding through `*args` and `**kwargs` costs in CPython
  too.
- Every `TypeError` such a call raises has CPython's message and comes
  from the same argument, so a program that catches it behaves alike.
- A positional tail with values of different types, such as `f(*numbers,
  "a")` into `*args: int | str`, is refused where a type checker accepts
  it.

## Related requirements

EXP-17, EXP-19, FUN-03, ADR-0060.
