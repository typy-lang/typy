# TyPy research: values whose type is known late

| Field | Value |
| --- | --- |
| Document | TyPy research: values whose type is known late |
| Version | 1.0 |
| Status | Approved |
| Owner | typy-lang contributors |
| Last updated | 2026-11-21 |

## 1. Purpose

This document asks which of the Python forms TyPy rejects are limits of
compiling ahead of time, and which are costs TyPy chose not to pay in its
first releases. It studies how statically typed languages hold values whose
type is not fixed by one annotation, measures what TyPy 0.10 accepts, and
proposes options for the requirements of 0.11 and later.

It builds on the research document, version 1.1, and on the software
requirements specification, version 1.113: the standard-Python rule
(STD-01 to STD-07), the rule that every value has one type (TYP-01, TYP-02,
TYP-10), and the features section 6.1 excludes because they "cannot be
compiled ahead of time with CPython's meaning". It keeps TyPy's aims:
Python's simplicity, native speed, and memory and type safety.

## 2. Method

The sources in section 11 were read on 2026-11-21. Three experiments ran
in the development container on the same day:

- **X3** runs 35 short programs, each using one language feature or one
  PEP, under CPython 3.12.16, 3.13.18, 3.14.10 and 3.15.2 (`python -I`),
  and checks each with TyPy 0.10.0 (tag `v0.10.0`) at the Python floor
  3.15.
- **X4** runs 12 programs that hold values of more than one type, or leave
  a type to inference, under CPython 3.15.2 and TyPy 0.10.0.
- **X5** reads the 752 standard library stubs of typeshed, as mypy 2.4.1
  bundles it, with Python's `ast` module, and counts how their parameters,
  results and classes are typed. The stubs are the types of the standard
  library that typed Python code is checked against.

Each X3 and X4 program is a few lines that use only the feature its row
names, such as `t"hi {name}"` for template strings. The X5 counter walks
the syntax tree of each stub, and a type counts as naming `Any` or `object`
when that name appears anywhere in it, also inside a union.

## 3. The question

TyPy compiles each value to one representation known when the program is
compiled: an `int` is a 64-bit word, a `str` a pointer to an object, a union
a tag and a slot per member (ADR-0027). That choice gives code as lean as
C's and lets the checker prove every operation. It is why TYP-10 rejects
`Any`, `cast` and `object`, and why section 6.1 rejects metaclasses,
`__getattr__`, `eval` and the rest.

The question is narrower than "static or dynamic". A type can be known at
three times:

1. **When the program is compiled**, from an annotation or from inference.
2. **When the program's definitions run**, at import: a metaclass, a class
   decorator, `type(name, bases, namespace)` or an `Enum` decides a class
   once, before any input is read.
3. **Only when the program runs**, from its input: `eval` of a string the
   user typed, an attribute named by a variable, a value of type `Any`.

Statically typed languages handle all three. They differ in what each costs
and where the cost is paid.

## 4. How statically typed languages hold values of any type

### 4.1 Java

| Form | Since | What it does |
| --- | --- | --- |
| `var` local variables | Java 10, 2018 (JEP 286) | The compiler infers a local's type from its initializer; fields, parameters and results keep their declared types |
| `Object` and boxing | Java 1.0, boxing in Java 5, 2004 | Every class derives from `Object`; an `int` stored in an `Object` is boxed as an `Integer` |
| Checked casts | Java 1.0 | `(String) value` checks the object's class and throws `ClassCastException` when it is wrong |
| `instanceof` patterns | Java 16, 2021 (JEP 394) | `if (value instanceof String text)` tests and narrows in one step |
| Patterns in `switch` | Java 21, 2023 (JEP 441) | A `switch` over an `Object` matches by class, as Python's `match` does |
| `invokedynamic` | Java 7, 2011 (JSR 292) | A call site whose target the program links when it runs, used by JRuby and Jython for dynamic languages |

Java is statically typed, yet a `List<Object>` holds values of any class,
and the compiler stays sound because every read of a narrower type is a
checked cast or a pattern.

### 4.2 C++

| Form | Since | What it does |
| --- | --- | --- |
| `auto` variables | C++11 | The type of a variable is inferred from its initializer |
| `auto` results | C++14 | A function's result type is inferred from its `return` statements |
| `auto` parameters | C++20 | `void f(auto x)` is a function template, compiled once for each type a call gives |
| `std::variant` | C++17 | A closed union with a tag, as TyPy holds `int \| str` |
| `std::any` | C++17 | A box for a value of any type; `std::any_cast` checks the type and throws `bad_any_cast` |
| `dynamic_cast` | C++98 | A checked downcast through run-time type information |
| `constexpr`, `consteval` | C++11, C++20 | Code the compiler runs while compiling, whose results are constants |

C++ shows two things TyPy can use: a parameter without a type can be a
template, compiled per call, and a program can run code inside the
compiler to build what it then compiles.

### 4.3 C\#

C# 3.0 (2007) added `var`. Every value converts to `object`, boxing a value
type. C# 4.0 (2010) added `dynamic`: a value whose operations are bound
when they run, through call-site caches, so code that uses `dynamic` pays
for the binding and every other line stays statically compiled.

### 4.4 Python compilers

- **mypyc** checks each annotated and each inferred type at run time where
  a value enters compiled code, and compiles `cast(str, x)` to a strict
  check that raises `TypeError`, where CPython does not check.
- **Codon** compiles an unannotated function once for each set of argument
  types the program uses, and infers result types, so plain Python
  functions need no annotations.
- **RPython**, the language PyPy is written in, lets the program use the
  full dynamism of Python while it initializes, including dynamic code
  generation. The translator then starts from the live objects that exist
  after initialization. Classes and functions cannot be defined after it.

### 4.5 Summary

| Technique | Example | What it would give TyPy |
| --- | --- | --- |
| Local inference | Java `var`, C++ `auto` | TyPy has it for locals (TYP-02) |
| Result inference | C++14 `auto` results | Functions without a return annotation |
| Parameters as templates | C++20 `auto` parameters, Codon | Functions without parameter annotations |
| A root type with boxing and checked narrowing | Java `Object`, C# `object`, C++ `std::any` | `object`, `list[object]`, `dict[str, object]` |
| Code run by the compiler | C++ `constexpr`, RPython initialization | Metaclasses, `Enum`, class decorators |
| Binding when the program runs | C# `dynamic`, Java `invokedynamic` | `Any`, `getattr(x, name)`, duck typing |

## 5. Where TyPy stands

### 5.1 X3: features and PEPs by CPython version

A cell is "yes" when that CPython runs the program.

| Feature | 3.12 | 3.13 | 3.14 | 3.15 | TyPy 0.10 |
| --- | --- | --- | --- | --- | --- |
| Type parameter defaults (PEP 696) | no | yes | yes | yes | Planned for 0.13 (GEN-06) |
| `typing.TypeIs` (PEP 742) | no | yes | yes | yes | Not provided, no requirement |
| `typing.ReadOnly` (PEP 705) | no | yes | yes | yes | Not provided, no requirement |
| `warnings.deprecated` (PEP 702) | no | yes | yes | yes | No `warnings` module |
| Annotations evaluated late (PEP 649) | no | no | yes | yes | Accepted |
| `except A, B:` (PEP 758) | no | no | yes | yes | Accepted |
| Template strings (PEP 750) | no | no | yes | yes | Rejected as excluded |
| Lazy imports (PEP 810) | no | no | no | yes | Accepted |
| `[*row for row in rows]` (PEP 798) | no | no | no | yes | Rejected by the parser, E0103 |
| `frozendict` (PEP 814) | no | no | no | yes | Not defined |
| `math.integer` (PEP 791) | no | no | no | yes | Not provided |
| Closed `TypedDict` (PEP 728) | no | no | no | yes | Rejected, no `TypedDict` |
| `typing.TypeForm` (PEP 747) | no | no | no | yes | Not provided |
| `Enum`, `Literal`, `TypedDict`, `overload`, `TypeGuard` | yes | yes | yes | yes | Not provided, no requirement |
| `abc.ABC` with `@abstractmethod` | yes | yes | yes | yes | Planned for 0.14 (FUN-15) |
| `functools`, `itertools`, `collections`, `re`, `json` | yes | yes | yes | yes | Not provided, no requirement |
| Complex numbers | yes | yes | yes | yes | Rejected (LEX-08) |
| `async` and `await`, `except*`, several bases | yes | yes | yes | yes | Planned for after 1.0 |

Two rejections do not match their reason. Template strings are rejected as
a form TyPy "cannot compile with CPython's meaning", yet a template string
evaluates to a `Template` object whose parts are fixed when it is built.
`Literal["r", "w"]` is rejected as "annotations written as strings", where
the cause is that TyPy does not provide `Literal`.

### 5.2 X4: inference and `object`

| Program | CPython 3.15 | TyPy 0.10 |
| --- | --- | --- |
| A local inferred from its first assignment | Runs | Runs (TYP-02) |
| A function without a return annotation | Runs | E0204 (TYP-01) |
| A function without parameter annotations | Runs | E0204 (TYP-01) |
| `items = []` then `items.append(1)` | Runs | E0410 |
| `def f(value: object)` narrowed by `isinstance` | Runs | E0403 (TYP-10) |
| `list[object]` matched with `match` | Runs | E0403 |
| `dict[str, object]` read and narrowed | Runs | E0403 |
| `isinstance(value, Box)` on an `object`, then `value.size` | Runs | E0403 |
| A method of `str` called on an `object` | Runs | E0403; mypy also rejects it |
| `typing.Any` | Runs | E0202 (TYP-10) |
| `typing.cast` | Runs | E0202 (TYP-10) |

Every program in X4 that mypy accepts in strict mode is statically
typable. TyPy rejects `object` because it has no representation for a
value of an open set of types, not because the checker cannot follow it.

### 5.3 X5: how the standard library is typed

| Measure | Count | Share |
| --- | --- | --- |
| Functions and methods | 17,307 | |
| Parameters other than `self` and `cls` | 29,978 | |
| Parameters whose type names `Any` | 1,669 | 5.6% of parameters |
| Parameters whose type names `object` | 907 | 3.0% of parameters |
| Parameters of type `object` alone | 452 | 1.5% of parameters |
| Results whose type names `Any` | 831 | 4.8% of results |
| Results whose type names `Self` | 806 | 4.7% of results |
| Definitions marked `@overload` | 2,558 | 14.8% of functions |
| Uses of `Literal` | 1,842 | |
| Classes | 3,246 | |
| `Protocol` classes | 250 | 7.7% of classes |
| `Enum` classes | 49 | 1.5% of classes |
| `TypedDict` classes | 31 | 1.0% of classes |
| Classes with a metaclass | 55 | 1.7% of classes |

The standard library's own types lean on forms TyPy 0.10 rejects or does
not provide: about one definition in seven is an overload, and `Any`,
`object`, `Self` and `Literal` each appear in hundreds of signatures. A
standard library written in typed Python and checked against these stubs,
as ADR-0007 decides, needs them first.

## 6. Real limits and chosen costs

Four limits hold whatever TyPy does:

1. **Behaviour Python does not define:** `id()` values, hash values, set
   order and when the collector runs. STD-03 leaves them out.
2. **Code that exists only when the program runs:** `eval` of a string
   built from input needs a compiler or an interpreter inside the program.
3. **CPython's C API:** running CPython's extension modules as they are
   needs CPython's object layout and C API, which ends TyPy's independence
   (ADR-0003).
4. **What a checker can prove:** some programs are correct but not
   provably typed, so a value the checker cannot follow needs a check when
   the program runs.

Every other exclusion is a cost:

| Rejected today | Time the type is known | Technique | Cost |
| --- | --- | --- | --- |
| `object` values | Compile, narrowed when running | A root type with boxing, as Java | A box for a value held by value |
| Missing result annotations | Compile | Result inference, as C++14 | Checker work |
| Missing parameter annotations | Compile | Parameters as templates, as C++20 and Codon | One copy per set of types; errors at the call that makes the copy |
| `items = []` before its first `append` | Compile | Inference from the first use, as mypy's partial types | Checker work |
| Metaclasses, `Enum`, class decorators, `type()` with three arguments | Definitions | Run the definitions in the compiler, as RPython and `constexpr` | Compile time; a rule for what the compiler may run |
| `__getattr__`, `__setattr__` | Compile | Call the method when a lookup fails or an attribute is set | None beyond the call |
| `globals()`, `locals()`, `vars()` | Running | A view of the variables, boxed | Boxing |
| `Any`, `getattr(x, name)` | Running | Binding when the program runs, as C# `dynamic` | Slower code where `Any` appears |
| `eval`, `exec` | Running | An interpreter of the same subset, linked only when used | Size and speed |
| `cast` | Compile, checked when running | A checked cast, as mypyc and Java | A program whose cast is wrong raises `TypeError` where CPython goes on |
| Template strings, complex numbers | Compile | Ordinary values | Runtime and checker work |

## 7. Options

### 7.1 Option J: `object` as a type, as Java's `Object`

A value of type `object` is the address of an object. A value held by
value, such as an `int`, a `float` or a tuple, is boxed when it becomes an
`object`, and unboxed when `isinstance`, `match` or a comparison narrows
it. Without narrowing, an `object` supports what `object` defines: `==`,
`!=`, `is`, `hash`, `str`, `repr`, truth and `type()`. `list[object]` and
`dict[str, object]` hold their items as objects.

- **Simplicity.** It accepts the common typed form `other: object` in
  `__eq__`, heterogeneous lists and JSON-shaped data, with mypy's rules.
- **Power.** Only values typed `object` pay for a box; every other value
  keeps its representation.
- **Safety.** Every narrowing reads the type ID in the object's header,
  which every TyPy object already carries (ADR-0012), so no operation can
  reach a value of the wrong type.
- **CPython.** It adds no meaning: CPython holds every value as an object.

### 7.2 Option I: more inference

- Infer a function's result from its `return` statements, as C++14 does,
  where every `return` gives one type or the union of them.
- Infer an empty collection's item type from its first use in the same
  function, as mypy does.
- Compile a function with unannotated parameters as a generic function, one
  copy per set of argument types, as C++20 and Codon do. TyPy already makes
  copies for generic functions and for protocol parameters (ADR-0023). An
  error inside such a function is reported at the call that makes the copy,
  which is harder to read than an error at an annotation.

Each part changes TYP-01, which requires every annotation today.

### 7.3 Option A: definitions run by the compiler

The compiler runs the code that defines classes and functions, where that
code reads no input and does no I/O, then compiles the classes it built:
`Enum` members, a metaclass's `__new__`, a class decorator, `setattr` on a
class at import. The result has no cost when the program runs. The design
needs a rule for what may run in the compiler, a sandbox for it, and a
proof that it gives the program CPython's meaning, which holds when the code
is deterministic.

### 7.4 Option B: a dynamic tier for `Any`

A value of type `Any` is an `object` whose operations bind when the program
runs: an attribute is looked up by name in a table the class emits, an
operator dispatches on the type IDs, and a call checks its arguments. A
wrong operation raises the `TypeError` or `AttributeError` CPython raises,
so memory safety holds. Only code that uses `Any` pays. Classes reached by
name lookup need name tables, which grow the binary.

### 7.5 Checked `cast`

`cast(T, value)` compiles to a check that raises `TypeError`, as mypyc and
Java do. This is the one form where TyPy would knowingly differ from CPython,
which trusts a wrong cast. STD-02 compares programs that CPython runs
without an uncaught exception, and a wrong cast is a program that the type
system calls wrong, so the difference needs an explicit decision.

## 8. Findings

1. Section 6.1 mixes four real limits with costs TyPy chose in 0.1. Its
   reason, "cannot be compiled ahead of time with CPython's meaning", holds
   for none of the costs.
2. Java, C++ and C# are statically typed and still hold values of any type,
   with boxing and checked narrowing. Option J is that model, and it is
   fully static.
3. TyPy rejects `object` for want of a representation, not for want of a
   rule: every X4 program mypy accepts is typable.
4. TyPy's runtime already has what option J needs: a type ID in every
   object's header, and `isinstance` and `match` against held classes
   (CLS-18).
5. TyPy's generic copies (ADR-0023, ADR-0081) are the mechanism option I
   needs for unannotated parameters.
6. Two rejections give the wrong reason: template strings, and `Literal`.
7. CPython 3.13 to 3.15 added forms with no requirement in TyPy, and the
   parser does not read PEP 798's unpacking in comprehensions.
8. Typeshed's standard library stubs use `@overload` in about one
   definition in seven, and `Any`, `object`, `Self` and `Literal` in
   hundreds of signatures, so these forms come before a standard library
   typed against the stubs.

## 9. Recommendation

Take the options in the order of their value and risk:

1. **J** first. It is static, matches CPython exactly, and opens the forms
   the 0.11 corpus measurement (NFR-24) will meet most often.
2. **I** next, starting with result inference and empty collections, then
   unannotated parameters once the diagnostics at a copy read well.
3. **A** for `Enum`, `abc` and class decorators, after a design of what the
   compiler may run.
4. **B** last, with the corpus figures showing how much code it would open.

Checked `cast` goes with J. The PEPs of section 5.1 become requirements,
each with the Python floor it needs, and template strings move from
excluded to planned.

## 10. Questions for the requirements

The requirements specification resolved these questions on 2026-11-21.

| ID | Question | Answer |
| --- | --- | --- |
| OI-32 | Is `object` a type, holding a boxed value narrowed by `isinstance` and `match` (option J), and in which release? | Yes, in 0.11 (TYP-23, ADR-0090) |
| OI-33 | Does TyPy infer results, empty collections and unannotated parameters (option I), which changes TYP-01? | Results and empty collections in 0.12; parameters keep their annotations (TYP-26, TYP-27, ADR-0092) |
| OI-34 | Which definition code may the compiler run, and how is CPython's meaning proved (option A)? | None yet: the checker expands `enum` and `abc` classes in 0.12, and option A stays for after 1.0 (CLS-44, CLS-45, ADR-0093) |
| OI-35 | Does TyPy accept `Any` as a value bound when the program runs (option B)? | Yes, in 0.13 (TYP-25, ADR-0094) |
| OI-36 | Does `cast` compile to a check that raises `TypeError`? | Yes, in 0.11 (TYP-24, ADR-0091) |
| OI-37 | Which CPython versions join the oracle after 3.15, and when does the lowest Python floor rise? | Each after its final release; the lowest floor rises after its end of life (STD-08, ADR-0095) |

## 11. References

| Source | Version or date | Location |
| --- | --- | --- |
| JEP 286, local-variable type inference | Java 10, 2018 | <https://openjdk.org/jeps/286> |
| JEP 394, pattern matching for `instanceof` | Java 16, 2021 | <https://openjdk.org/jeps/394> |
| JEP 441, pattern matching for `switch` | Java 21, 2023 | <https://openjdk.org/jeps/441> |
| JSR 292, `invokedynamic` | Java 7, 2011 | <https://jcp.org/en/jsr/detail?id=292> |
| C++ standard, `auto`, `std::any`, abbreviated templates | C++11, C++17, C++20 | <https://en.cppreference.com/w/cpp/language/auto> |
| C# `dynamic` | C# 4.0, 2010 | <https://learn.microsoft.com/dotnet/csharp/advanced-topics/interop/using-type-dynamic> |
| mypyc, differences from Python | Read 2026-11-21 | <https://mypyc.readthedocs.io/en/latest/differences_from_python.html> |
| Codon language guide | Repository read 2026-11-21 | <https://github.com/exaloop/codon/tree/develop/docs/language> |
| PyPy coding guide, RPython | Read 2026-11-21 | <https://doc.pypy.org/en/latest/coding-guide.html> |
| typeshed standard library stubs | As bundled with mypy 2.4.1 | <https://github.com/python/typeshed> |
| PEPs 649, 696, 702, 705, 728, 742, 747, 750, 758, 791, 798, 810, 814 | Python 3.13 to 3.15 | <https://peps.python.org/> |
| TyPy research | 1.1, 2026-11-05 | [research.md](research.md) |
| TyPy software requirements specification | 1.113, 2026-11-21 | [srs.md](../requirements/srs.md) |
