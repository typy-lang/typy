# TyPy research: newer Python and a typed standard library

| Field | Value |
| --- | --- |
| Document | TyPy research: newer Python and a typed standard library |
| Version | 0.1 |
| Status | Draft |
| Owner | typy-lang contributors |
| Last updated | 2026-11-21 |

## 1. Purpose

This document studies the forms of typed Python that TyPy 0.10 rejects and
no requirement plans yet: template strings, the typing forms the standard
library's own types use, the language changes of CPython 3.15, and the
standard library itself. For each it measures what CPython does, what
TyPy lacks, and what the work needs first, and it proposes questions for
the requirements.

It builds on the research note "values whose type is known late",
version 1.0, whose experiment X3 found these gaps, and on the software
requirements specification, version 1.115, with its rule for following
CPython versions (STD-08, ADR-0095) and ADR-0007, which decides that the
standard library is typed Python compiled by TyPy.

## 2. Method

Two experiments ran in the development container on 2026-11-21:

- **X7** checks how TyPy 0.10.0 (tag `v0.10.0`) reads template strings and
  PEP 798's unpacking in comprehensions: what its parser, the pinned
  `ruff_python_parser` 0.0.20 of 2026-10-27, accepts, and what TyPy's own
  subset check rejects. CPython 3.14.10 and 3.15.2 give the expected
  results.
- **X8** reads typeshed's stub of each of 32 standard library modules, as
  mypy 2.4.1 bundles it, and counts its functions, its classes and the
  uses of each typing form in it.

## 3. Template strings

PEP 750 adds template strings to CPython 3.14: `t"hi {name!r:>5}"`
evaluates to a `string.templatelib.Template`, not a `str`. Under CPython
3.15.2 that template has `strings` equal to `('hi ', '')`, one
`Interpolation` whose `value`, `expression`, `conversion` and
`format_spec` are `'x'`, `'name'`, `'r'` and `'>5'`, and iterates as
`['hi ', Interpolation('x', 'name', 'r', '>5')]`. Every part is fixed
when the expression runs.

TyPy 0.10 classes template strings as a form it cannot compile with
CPython's meaning (E0202). X7 finds the parser reads them, and nothing in
them is dynamic: a template is an immutable object of strings and
interpolations, as a tuple of fixed shape is. The class is wrong.

A template string is a syntax error under CPython 3.12 and 3.13, so a
program uses one only with a Python floor of 3.14 or later (STD-01).

## 4. Typing forms the standard library uses

The first research note's X5 counted typeshed's 752 standard library
stubs: about one definition in seven is an `@overload`, 806 results name
`Self`, and `Literal` is used 1,842 times. X8 shows where each form
matters:

| Module | Functions | Classes | `@overload` | `Self` | `Literal` | `Any` | `Callable` | `Final` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `textwrap` | 14 | 1 | 0 | 0 | 0 | 0 | 1 | 0 |
| `base64` | 37 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `heapq` | 7 | 0 | 6 | 0 | 0 | 0 | 3 | 1 |
| `statistics` | 47 | 4 | 0 | 1 | 3 | 0 | 2 | 0 |
| `time` | 37 | 2 | 0 | 0 | 1 | 1 | 0 | 14 |
| `random` | 32 | 2 | 2 | 0 | 0 | 9 | 1 | 0 |
| `string` | 17 | 2 | 4 | 0 | 0 | 18 | 0 | 9 |
| `copy` | 5 | 2 | 0 | 0 | 0 | 4 | 0 | 0 |
| `itertools` | 105 | 19 | 54 | 31 | 22 | 4 | 4 | 0 |
| `collections` | 269 | 14 | 84 | 73 | 0 | 21 | 8 | 0 |
| `datetime` | 137 | 7 | 9 | 39 | 0 | 0 | 0 | 2 |
| `pathlib` | 122 | 7 | 19 | 39 | 4 | 2 | 1 | 0 |
| `os.path` (`posixpath`) | 55 | 0 | 49 | 0 | 0 | 0 | 0 | 0 |
| `fractions` | 79 | 2 | 51 | 8 | 1 | 1 | 1 | 0 |
| `re` | 82 | 4 | 56 | 0 | 2 | 8 | 10 | 18 |
| `json` | 7 | 0 | 0 | 0 | 1 | 56 | 24 | 0 |
| `functools` | 57 | 11 | 19 | 5 | 18 | 39 | 51 | 5 |
| `argparse` | 140 | 30 | 24 | 2 | 0 | 67 | 10 | 6 |
| `logging` | 122 | 19 | 7 | 1 | 1 | 14 | 7 | 10 |
| `enum` | 89 | 18 | 6 | 41 | 1 | 31 | 2 | 7 |

`bisect` and `struct` re-export C modules, so their stubs hold no
functions of their own.

What each form needs:

- **`Literal`** narrows a type to listed constants. When the program runs
  the value is of the base type, so the work is in the checker: a
  constant passes for its literal type, and `==` and `match` narrow.
- **`@overload`** gives a function several signatures and one body. The
  checker picks the signature a call matches, and checks the body against
  each; the function compiles once.
- **`Self`** is the class a method is called through. The checker reads it
  as the receiver's type; the runtime needs nothing. SRS section 6.2 plans
  it for after 1.0 with exception groups, though it is the cheaper of the
  two.
- **`TypeGuard` and `TypeIs`** let a function returning `bool` narrow its
  argument, as `isinstance` does.
- **`TypedDict`** is a dict whose keys are fixed and whose values each
  have a type. When the program runs it is a plain `dict`, so its values
  are held as `object` (TYP-23) and read with the key's type.
- **`Any`** is planned for 0.13 (TYP-25), and `Final` is accepted.

`Literal["r", "w"]` is rejected today with the message for annotations
written as strings, which hides the cause: TyPy does not provide
`Literal`.

## 5. CPython 3.15

- **PEP 798, unpacking in comprehensions.** `[*row for row in rows]`
  flattens, and `{**part for part in parts}` merges. X7 finds the parser
  reads both at the floor 3.15; TyPy's subset check rejects the starred
  element with CPython's syntax error for older versions. The work is in
  the checker and the lowering, which extend the comprehension's result
  by each item's items.
- **PEP 814, `frozendict`.** An immutable, hashable mapping derived from
  `object`, with `copy`, `fromkeys`, `get`, `items`, `keys` and `values`,
  and `|` that returns a new `frozendict`. TyPy can hold it as a dict that
  refuses changes, as it holds a tuple as a list that refuses them.
- **PEP 791, `math.integer`.** A module of `comb`, `factorial`, `gcd`,
  `isqrt`, `lcm` and `perm`, each already provided by `math`, so the work
  is a module table that names them (ADR-0018).

Each is accepted only with a Python floor of 3.15 (STD-01, STD-08).

## 6. A typed standard library

ADR-0007 decides that each standard library module is typed Python that
TyPy compiles and CPython runs, with the operating system reached through
a private module `_typy`. No such module exists yet: `math`, `sys`,
`typing`, `dataclasses` and `ctypes` are tables in the checker
(ADR-0018), so each new module today means compiler work.

X8 groups the candidate modules by what they need beyond TyPy 0.10:

| Group | Modules | Needs |
| --- | --- | --- |
| 1 | `textwrap`, `base64`, `heapq`, `bisect`, `statistics`, `time`, `random`, `copy`, `string` | The `typy-stdlib` crate, `_typy`, and `object` (TYP-23) for a few signatures |
| 2 | `itertools`, `collections`, `datetime`, `pathlib`, `os.path`, `fractions`, `re` | Group 1, `@overload`, `Self` and `Literal` |
| 3 | `json`, `functools`, `argparse`, `logging`, `csv` | Group 2, `Any` (TYP-25) and `**P` (GEN-06) |
| 4 | `enum`, `abc` | The expansions of CLS-44 and CLS-45 |

The signatures come from typeshed, the types every typed Python program is
checked against. Reading typeshed's stubs when a program compiles would
need every form they use; checking each TyPy module's signatures against
typeshed in a test, as mypy's `stubtest` compares stubs with modules, keeps
the two in step without that.

## 7. Findings

1. Template strings are classed wrongly: they are immutable objects fixed
   when the expression runs, and TyPy's parser reads them.
2. `@overload`, `Self` and `Literal` gate most of the standard library's
   second group of modules, and need only checker work.
3. The parser TyPy pins already reads PEP 798; the rejection is TyPy's
   own.
4. `frozendict` and `math.integer` reuse what TyPy has: an immutable
   container and the `math` table.
5. The first group of standard library modules needs no typing form TyPy
   lacks beyond `object`; it needs the crate and `_typy` that ADR-0007
   decided and that do not exist yet.
6. `Literal`'s message names the wrong cause.

## 8. Questions for the requirements

The requirements specification holds these questions as open issues.

| ID | Question |
| --- | --- |
| OI-38 | In which release do template strings become a planned feature, from the floor 3.14? |
| OI-39 | In which releases do `Literal`, `@overload`, `Self`, `TypeGuard`, `TypeIs` and `TypedDict` come, and does `Self` move from after 1.0? |
| OI-40 | In which release do PEP 798's unpacking in comprehensions, `frozendict` and `math.integer` come, from the floor 3.15? |
| OI-41 | In which release does the `typy-stdlib` crate start, with which modules first, and are its signatures checked against typeshed? |

## 9. References

| Source | Version or date | Location |
| --- | --- | --- |
| PEP 750, template strings | Python 3.14 | <https://peps.python.org/pep-0750/> |
| PEP 798, unpacking in comprehensions | Python 3.15 | <https://peps.python.org/pep-0798/> |
| PEP 814, `frozendict` | Python 3.15 | <https://peps.python.org/pep-0814/> |
| PEP 791, `math.integer` | Python 3.15 | <https://peps.python.org/pep-0791/> |
| Ruff pull request 25104, PEP 798 support | Merged 2026-05-16 | <https://github.com/astral-sh/ruff/pull/25104> |
| `ruff_python_parser` | 0.0.20, 2026-10-27 | <https://crates.io/crates/ruff_python_parser> |
| typeshed standard library stubs | As bundled with mypy 2.4.1 | <https://github.com/python/typeshed> |
| mypy `stubtest` | mypy 2.4.1 | <https://mypy.readthedocs.io/en/stable/stubtest.html> |
| TyPy research: values whose type is known late | 1.0, 2026-11-21 | [static-and-dynamic-values.md](static-and-dynamic-values.md) |
