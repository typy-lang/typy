# ADR-0101: Keep the hash of a `str` in the object

## Status

Accepted

## Date

2026-11-22

## Context

NFR-21 asks that no program of the benchmark suite built with `--release`
run slower than CPython 3.15. The full run of 2026-11-22 put knucleotide
at 1.346 s against CPython's 1.329 s, with overlapping ranges. Copying
the slices of one-byte strings as runs of bytes, and hashing and comparing
`str` keys without looking up their type, bring it to 0.85 of CPython's
time.

knucleotide counts the fragments of a sequence in a dict:
`found[fragment] = found.get(fragment, 0) + 1`, over a new slice of the
text each time. A TyPy `str` keeps no hash, so each fragment is hashed
twice with SipHash-1-3, once by `get` and once by the store. CPython keeps
the hash of a `str` in the object and hashes the fragment once.

## Options considered

1. **Keep the hash in the object.** A field after `kind` holds the hash,
   or -1 until the string is first hashed. The fixed part of a `str`
   grows from 24 bytes to 32.
2. **Hash once in the compiler** for the form `d[k] = d.get(k, x) + y`.
   This helps that form only: a program that writes
   `if k in d: d[k] += 1` still hashes twice.
3. **Keep the layout.** knucleotide stays at 0.85 of CPython's time, and
   a string used as a key in several lookups is hashed for each.

## Decision

Option 1. `StrObject` gains `hash: AtomicI64` at offset 24, and the data
starts at offset 32. `new_string` writes -1, which no hash is. The first
hash of a string stores its value with a relaxed store. Two threads that
hash one string at once store the same value, so the order of the stores
does not matter, and the atomic keeps the access defined for the threads
of ADR-0096. An immortal string is never written: the constants the
compiler emits and the runtime's shared strings live in read-only memory,
so their hash is computed each time.

## Consequences

- A fragment that is looked up and then stored is hashed once.
  knucleotide runs at 0.80 of CPython's time, against 0.85 without the
  kept hash (medians of five alternating runs at size 1,000,000, pinned to
  one core).
- Each `str` and `bytes` takes 8 more bytes.
- The compiler's constant `str` and `bytes` objects carry the field, and
  the layout in detailed design section 14.1 changes with them. `typy`
  links only a runtime archive of its own version (E0703), so a constant
  of the older layout never meets the new runtime.
- C code that writes into the buffer of a `bytes` after the `bytes` is
  hashed leaves a stale hash, as it does in CPython.

## Related requirements

NFR-21, NFR-12, FFI-03, ADR-0010, ADR-0096, ADR-0100.
