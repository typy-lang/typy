# ADR-0093: Expand `enum` and `abc` classes in the checker

## Status

Accepted

## Date

2026-11-21

## Context

`enum.Enum` and its kinds, and `abc.ABC` with `@abstractmethod`, are
common in typed Python: typeshed's standard library stubs hold 49 enum
classes and 55 classes with a metaclass. CPython builds both with
metaclasses (`EnumType`, `ABCMeta`), which TyPy rejects (SRS section 6.1).

The research note "values whose type is known late" weighed running the
code that defines classes inside the compiler, as RPython and C++
`constexpr` do, against expanding the classes TyPy knows, as the checker
already expands `@dataclass` (CLS-05) and `NamedTuple` (ADR-0088)
(OI-34).

## Options considered

1. **Run definition code in the compiler.** General, but needs an
   evaluator inside the compiler, a sandbox, and a proof that each piece
   of code it runs is deterministic.
2. **Expand the classes of `enum` and `abc` in the checker,** with the
   behaviour CPython's metaclasses give them.
3. **Leave them for after 1.0.**

## Decision

Option 2. Running definition code in the compiler stays a question for
after 1.0.

- **Enums.** A class derived from `Enum`, `IntEnum`, `StrEnum`, `Flag` or
  `IntFlag` is expanded into a class whose members are immortal instances
  made when the class is defined, each with `name` and `value`. The class
  is iterable in definition order, has `len`, finds a member by value
  with a call and by name with a subscript, raising CPython's
  `ValueError` and `KeyError`, and gives `auto()` the values CPython
  gives. Members compare and hash as CPython's do, an `IntEnum` or
  `StrEnum` member also as its value, and `Flag` members combine with `|`,
  `&` and `^`. A member's type is the class, and `match` takes members as
  value patterns.
- **Abstract classes.** A class derived from `abc.ABC` may mark methods
  `@abstractmethod`. Making an instance of a class that leaves one of
  them without an override is E0404 where the class is named, and raises
  CPython's `TypeError` through a `type[C]` value the checker cannot
  decide.
- **Members are checked as classes are:** an enum's methods and
  properties, and an abstract method's signature, follow the class rules.

## Consequences

- No evaluator enters the compiler, and each expansion is tested against
  CPython's module, as the dataclass and named-tuple expansions are.
- Other metaclasses and class-building code stay rejected; each new kind
  needs an expansion of its own.
- `@abstractmethod` is accepted before the other method decorators
  (FUN-15).

## Related requirements

CLS-44, CLS-45, CLS-05, CLS-27, FUN-15, OI-34, ADR-0088.
