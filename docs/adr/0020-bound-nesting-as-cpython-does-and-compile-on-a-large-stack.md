# ADR-0020: Bound nesting as CPython does and compile on a large stack

## Status

Accepted

## Date

2026-11-13

## Context

The parser and every pass of the compiler recurse over the syntax tree:
converting it, checking, lowering and generating code. Deep nesting
therefore uses deep stack. A fuzzed input with about 45 nested braces
overflowed the parser's stack under AddressSanitizer, and the `typy`
command itself aborted on 1000 nested brackets, a 2 KB program, against
NFR-09.

CPython bounds nesting itself. Its tokenizer refuses brackets nested more
than 200 deep, "too many nested parentheses", and blocks more than 99
deep, "too many levels of indentation". Its parser and compiler refuse
deeper expressions with `MemoryError` or `RecursionError`. Measured with
`compile()`:

| Shape | CPython 3.12 | CPython 3.15 |
| --- | --- | --- |
| Brackets, calls | 200 | 200 |
| Blocks | 99 | 99 |
| Unary operators, `not` | 5966 | 5965 |
| `a if c else b` chains | 5966 | 2982 |
| `+` chains, attribute chains | 9995 | 22685 |
| Comparison and `and` chains | over 30000, flat | over 30000, flat |

The TyIR verifier also computed, for every block, the set of blocks that
dominate it, which costs memory in the square of the number of blocks: a
chain of 20000 `elif` clauses ran out of 6 GB, and 5966 nested `if`
expressions took 87 seconds.

## Options considered

1. **Refuse brackets past 200 only.** Matches CPython for brackets, but
   unary operators, `if` expressions and `elif` chains nest without
   brackets and still overflow.
2. **Rewrite every pass without recursion.** Removes the limit on the
   stack, at the cost of rewriting the converter, the checker, lowering
   and code generation.
3. **Bound nesting and size the stack for the bound.** Refuse what
   CPython's tokenizer refuses, with its messages; let the parser grow its
   stack on the heap (the `stacker` feature of `ruff_python_parser`); cut
   off expressions and statements nested past a depth deeper than any
   program CPython compiles; and run the compiler on a thread whose stack
   holds that depth.

## Decision

TyPy uses option 3 (LEX-11).

- Before parsing, a pass over the tokens refuses the 201st nested
  bracket with "too many nested parentheses" and the 100th nested block
  with "too many levels of indentation", as E0103.
- The parser grows its stack on the heap, so no input overflows it.
- Converting the parser's tree counts the nesting of expressions and
  statements, an `elif` counting as the `if` it becomes, and stops past
  `MAX_DEPTH`, 25000, with "the program nests more than 25000 levels
  deep, past what CPython compiles". 25000 is past the deepest program
  any oracle version compiles.
- `typy` runs the compiler on a thread with a 512 MiB stack. The memory
  is reserved, and only what a program's nesting uses is taken.
- The verifier finds immediate dominators with Cooper, Harvey and
  Kennedy's iteration, and answers whether a block dominates another
  from a numbering of the dominator tree, in constant time.

## Consequences

- Every program CPython compiles, compiles; a deeper one is refused with
  E0103, never a crash. A program 22685 levels deep builds in about 2
  seconds.
- Code that calls the stages in its own thread, such as a test or the
  fuzz harness, gives that thread the same large stack to convert a tree
  nested `MAX_DEPTH` levels.
- `stacker` and `psm` join the compiler's dependencies; the runtime is
  unchanged.
- A function of many blocks verifies in time and memory in proportion to
  them.

## Related requirements

LEX-11, NFR-09, STD-01, OI-03, ADR-0005.
