# ADR-0010: Hash and order sets and dicts as CPython does

## Status

Accepted

## Date

2026-11-09

## Context

Release 0.2 adds `dict[K, V]` and `set[T]` (DAT-09, DAT-10). A program sees
two things a hash table decides: the order in which a `set` is iterated and
printed, and the values `hash()` gives. A `dict` iterates in insertion
order, whatever the hashes, so only its speed depends on them.

CPython iterates a `set` in the order of the slots of its hash table. That
order follows from the hash of each item, the table's size, the probe
sequence (nine linear probes, then a perturbed jump) and when the table
grows, and from the dummy entries removals leave. `print({5, 1, 9, 33})`
writes `{1, 33, 5, 9}`, and every CPython from 3.12 to 3.15 writes it the
same way. The hashes of `int`, `float`, `bool`, `None` and tuples are fixed
by the language: an `int` is reduced modulo the Mersenne prime 2^61 - 1, a
`float` by the same rule on its exact value, `hash(None)` is a constant
since 3.12, and a tuple combines its items' hashes with xxHash's rounds.
The hashes of `str` and `bytes` are SipHash with a key chosen at random when
CPython starts, so CPython itself orders a set of strings differently from
one run to the next, unless `PYTHONHASHSEED` fixes the key. The conformance
suite runs CPython without it.

A NaN hashes by its identity since CPython 3.10, so equal-looking NaN items
are kept apart; TyPy's floats have no identity, the question OI-18 asks.

## Options considered

1. **A table of TyPy's own.** Any good hash and table design. Sets of
   numbers would print in an order no CPython gives, so a program's output
   would differ from CPython's, against ADR-0008.
2. **Sort a set when it is printed or iterated.** The output would be stable
   but still not CPython's, and sorting costs time on every iteration.
3. **CPython's hashes and CPython's set algorithm.** TyPy computes the hashes
   CPython computes for `int`, `float`, `bool`, `None` and tuples, and its
   set table repeats CPython's: the initial size of eight slots, the probe
   sequence, the load factor at which it grows, the size it grows to, and
   the dummy entries. A `dict` is a compact table, an array of entries in
   insertion order with an index of slots, as CPython's is. `str` and
   `bytes` hash with SipHash-1-3, the function CPython uses, under a key
   fixed in the runtime, since any key gives an order CPython could give.

## Decision

TyPy uses option 3. The runtime holds `dict` and `set` as ADR-0009 holds
lists: once, generic over the layout of their keys, values and items, with
the item types and the descriptors ADR-0001 and ADR-0009 define, and a hash
function for each kind of item.

## Consequences

- A set of numbers, booleans, `None` or tuples of them iterates and prints
  as CPython orders it, so the conformance suite compares such programs
  with CPython directly.
- A set of strings or `bytes` iterates in an order CPython could give, but
  not the one any one CPython run gives. A conformance test does not print
  such a set unordered; it sorts it, or tests membership.
- The set algorithm must follow CPython's `setobject.c` closely, and a
  change to it in a later CPython would show as a test whose oracles
  disagree; the oracles from 3.12 to 3.15 agree today.
- `hash()` of a number or a tuple gives CPython's value, which lets `hash()`
  join the builtins with the same results.
- A NaN is never equal to another item, OI-18 decides, so a set keeps each NaN
  apart, as CPython keeps NaN objects made separately.

## Related requirements

DAT-09, DAT-10, LIB-03, ADR-0001, ADR-0008, ADR-0009, OI-18.
