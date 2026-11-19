# Functions

## What TyPy accepts

| Item | Rule |
| --- | --- |
| Function definitions at the top of the module, with annotated parameters and return | FUN-01, TYP-01 |
| Recursion; running out of stack ends the program with `RecursionError` | FUN-02 |
| Positional, keyword and default arguments, defaults evaluated once | EXP-16 |
| Positional-only parameters before `/` and keyword-only parameters after `*`, bound as CPython binds them; a keyword for a positional-only parameter, or a positional argument past the others, is E0404 | FUN-03 |
| A function of the program as a value of type `Callable[[A, ...], R]`: stored, passed, returned, called, compared and hashed | FUN-05, NFR-19 |
| A method read through an instance as a bound method, and through its class as a function | FUN-05 |
| A function defined inside a function, which reads the variables of the functions around it | FUN-04 |
| A generator function, whose `yield` statements give its items one step at a time | SMT-11, FUN-07 |
| `send`, `throw` and `close` of a `Generator[Y, S, R]`, and `received = yield value` | SMT-11, FUN-07 |

A function value is the `key` of `sorted`, `list.sort`, `min` and `max`;
see [Ordering by a key](../library/functions-and-modules.md#ordering-by-a-key).
Decorators, built-in functions as values, class methods and protocol
methods as values, and nested functions of generic functions are planned
for 0.10 (FUN-06, FUN-08 to FUN-10).

## Nested functions

A `def` inside a function defines a nested function, which reads the
variables of the functions around it. Each time the `def` runs it makes a
new function object, so two calls of `make_adder` below give two
functions that are not `==`, as in CPython:

<!-- typy-expect: accepted -->
```python
from typing import Callable


def make_adder(step: int) -> Callable[[int], int]:
    def add(value: int) -> int:
        return value + step
    return add


def countdown(start: int) -> list[int]:
    def walk(value: int) -> list[int]:
        return [0] if value == 0 else [value] + walk(value - 1)
    return walk(start)


add3 = make_adder(3)
print(add3(1), make_adder(5)(1), add3 == make_adder(3), countdown(3))
```

A variable that one assignment gives its value, outside every loop and
before the `def`, is copied into the function object when the `def` runs,
so reading it costs what reading a local costs. The function object holds
a reference to each value it copies, which lives as long as the function
does. A nested function's own name reads the running function, so it may
call itself, and it calls the other nested functions of the function
around it by name. Its signature follows the rules of a module's
functions, keyword-only parameters included, which a call by name passes;
such a function has no `Callable` type, so reading it as a value is E0202.

A variable that a loop binds, or that is assigned again or after the
`def`, lives in a cell that the function and its nested functions share,
so a nested function reads its value when it runs, as in CPython. The
functions made in the loop below all read the last value of `i`, and
`read` sees `total` assigned after its `def`:

<!-- typy-expect: accepted -->
```python
from typing import Callable


def made_in_a_loop() -> list[int]:
    functions: list[Callable[[], int]] = []
    for i in range(3):
        def get() -> int:
            return i
        functions.append(get)
    return [each() for each in functions]


def assigned_later() -> int:
    def read() -> int:
        return total * 2
    total = 21
    return read()


print(made_in_a_loop(), assigned_later())
```

Nested functions that call one another by name, defined one after
another and each bound only by its `def`, such as `even` and `odd` below,
hold the same captures: each holds every variable any of them reads, and a
call between them passes the running function along. No function object
holds another, so each is freed with its last reference:

<!-- typy-expect: accepted -->
```python
def parity(n: int) -> str:
    def even(k: int) -> bool:
        return True if k == 0 else odd(k - 1)

    def odd(k: int) -> bool:
        return False if k == 0 else even(k - 1)
    return "even" if even(n) else "odd"


print(parity(7), parity(10))
```

Two such functions with another statement between their `def`s, or one
that reads the other as a value rather than calling it, run as in CPython
but hold each other. That reference cycle is never freed until cycle
collection, which is planned for 1.0 (MEM-06).

A nested function that reads a variable before the function around it
assigns one raises `NameError`, with CPython's message. A nested function
assigns a variable of the function around it after declaring it
`nonlocal`, described in
[Simple statements](simple-statements.md). Binding the name of a nested
function again,
also with a second `def`, is E0202, and a call of it before its `def`
runs is E0306, where CPython raises `UnboundLocalError`.

## Generators

A function whose body has a `yield` is a generator function. A call of it
runs none of its body: it gives a generator object, and each step, a turn
of a `for` loop or a call of `next()`, runs the body from where it last
stopped to its next `yield`, whose value is the step's item. The
annotation names the type each `yield` gives, as `typing` reads it:
`Iterator[T]`, `Iterable[T]` or `Generator[T, S, R]`, from
`collections.abc` or `typing`. A generator is a value of that type, which
`for`, `next()`, `list()`, `sum()`, `sorted()` and every function that
takes an iterable accept:

<!-- typy-expect: accepted -->
```python
from collections.abc import Iterator


def countdown(start: int) -> Iterator[int]:
    while start > 0:
        yield start
        start -= 1


def walk(depth: int) -> Iterator[str]:
    yield "level " + str(depth)
    if depth > 0:
        for line in walk(depth - 1):
            yield "  " + line


print(list(countdown(3)), sum(countdown(4)))
for line in walk(2):
    print(line)
```

The generator's variables live in the generator object between steps,
one allocation whatever the body does, and its `repr` is `<generator
object NAME at 0x...>`, as in CPython. `next()` of a finished generator
raises `StopIteration`; `next()` of a generator from inside its own step
raises `ValueError`, and a `StopIteration` that escapes the body becomes
`RuntimeError`, as CPython has it. A generator function annotated any
other way is E0405, and a `yield` outside a function is E0103.

### Sending values to a generator

A generator annotated `Generator[Y, S, R]` has the methods of CPython's
generators. `send(value)` resumes it with a value of type `S`, which an
assignment of its `yield` receives, and returns the next item;
`throw(exception)` raises the exception at the `yield` it stopped at;
`close()` raises `GeneratorExit` there, so its `finally` blocks run. An
`Iterator[Y]` or an `Iterable[Y]` has none of them, so a generator
function you drive with `send` is annotated `Generator`:

<!-- typy-expect: accepted -->
```python
from collections.abc import Generator


def running_total() -> Generator[int, int, None]:
    total = 0
    while True:
        received = yield total
        if received is not None:
            total += received


totals = running_total()
print(next(totals))
print(totals.send(5), totals.send(10))
totals.close()
```

`next()` and a `for` loop resume a generator with `None`, and so does
`send(None)`, so the `yield` of `received = yield total` gives
`int | None`, and the program tests for `None` before it adds. Type
checkers give it the type `int`; TyPy gives the type that holds every
value CPython can give it (ADR-0067). A variable declared `int` that the
`yield` is assigned to is E0402. `send` takes an `S` only, as type
checkers have it, so a generator of `Generator[int, int, None]` starts
with `next()`; `send(None)` to it is E0404. A value other than `None`
sent before the first step raises `TypeError`, and `send` or `throw`
after the body ends raises `StopIteration`, as in CPython.

A generator suspended inside an `except` clause keeps the exception it
handles: its caller does not see it, and the generator handles it again
when it resumes, as CPython does (ADR-0068).

A `yield` is a statement in TyPy: `yield value`, `yield`, and
`target = yield value`, also with an annotation. One inside a larger
expression, such as `print((yield value))`, and a generator `lambda` are
planned for 1.0 (FUN-12).

## Functions as values

A function's name read without a call is its function object, a value of
type `Callable[[A, ...], R]` of `typing` or `collections.abc`, where `A,
...` are its parameter types and `R` its result type. A variable, a list
item, a dict key, an attribute or an argument holds it, and a call through
it passes one argument by position for each parameter:

<!-- typy-expect: accepted -->
```python
from typing import Callable


def double(value: int) -> int:
    return value * 2


def square(value: int) -> int:
    return value * value


def apply(function: Callable[[int], int], value: int) -> int:
    return function(value)


operations = [double, square]
names = {double: "double", square: "square"}
print([apply(each, 3) for each in operations], names[square])
print(double is operations[0], double == square)
```

As in CPython, each function has one object, so `is`, `==` and `hash`
compare functions by identity, a function value is true, and `repr` writes
`<function double at 0x...>` with the object's address. A function value
has no order, so `<` between two is E0407.

A `lambda` is a function value too, made where a `Callable` is expected,
such as `apply(lambda x: x + 1, 3)`; its `repr` names it `<lambda>`. It
is described in [Expressions](expressions.md).

A function fits a `Callable` only when a call through the value converts
nothing (NFR-19). Each argument the `Callable` passes must be a value of
the function's parameter type as it is held, and the function's result a
value of the `Callable`'s result type as it is held. The types may differ
only by a class of the program, since every object is held as its
address: a function of any `Shape` fits a `Callable[[Circle], None]`, and
a function that returns a `Circle` fits a `Callable[[], Shape]`. A
function of an `int | None` does not fit a `Callable[[int], int]`, since
a call by name holds the `int` as an optional value first, and a function
of a `float` does not fit a `Callable[[int], float]`, since an `int` is
never a `float` (TYP-09):

<!-- typy-expect: rejected E0402 at 8:37 -->
```python
from typing import Callable


def scale(value: float) -> float:
    return value * 2.0


converter: Callable[[int], float] = scale
```

A call through a value checks the stack, as a recursive function does, so
recursion through function values ends with `RecursionError` too (FUN-02).

## Methods as values

A method read through an instance, without a call, is a bound method: a
new object at each read that holds the instance, of type
`Callable[[A, ...], R]` of the parameters after `self`. As in CPython it
binds the method of the object's own class, so a subclass's override
runs, two bound methods of one instance and one method are `==` and hash
alike while `is` tells them apart, and `repr` writes
`<bound method Class.name of ...>`, the instance written as its class
writes it. A method read through its class is the function itself, with
the instance as its first parameter, and a static method is the function
it is:

<!-- typy-expect: accepted -->
```python
from typing import Callable


class Counter:
    def __init__(self) -> None:
        self.count = 0

    def add(self, step: int) -> int:
        self.count += step
        return self.count


class Double(Counter):
    def add(self, step: int) -> int:
        self.count += 2 * step
        return self.count


def run(step: Callable[[int], int]) -> int:
    return step(1) + step(1)


counter: Counter = Double()
print(run(counter.add), counter.count, counter.add == counter.add)
print(Counter.add(counter, 10), type(counter.add))
```

A bound method holds a reference to its instance, so the instance lives
as long as the bound method does.

These forms are refused:

| Form | Diagnostic |
| --- | --- |
| `Callable[..., R]`, which takes any arguments, and `Callable` without its types | E0408 |
| A keyword argument through a function value, such as `f(value=1)`, since a `Callable` names no parameters | E0404 |
| A generic function as a value, which has one copy for each set of types | E0202 |
| A function with a keyword-only parameter as a value, since a `Callable` passes every argument by position | E0202 |
| A method with type parameters of its own, or with a keyword-only parameter, as a value | E0202 |
| An assignment to a method of an instance, such as `box.close = other`, since TyPy does not replace a class's attributes when the program runs | E0414 |
| A class method as a value; a static method read through an instance whose subclasses define it again; a method read through a protocol value; a method read through an instance whose subclass overrides it with parameters or a result held otherwise, planned for 0.10 | E0201 (FUN-09) |
| A call of a value that is not a function, such as an `int` | E0401 |

## Example

<!-- typy-expect: accepted -->
```python
def fib(n: int) -> int:
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


def greet(name: str, punctuation: str = "!") -> str:
    return "hello " + name + punctuation


print(fib(20), greet("world"), greet("you", punctuation="?"))
```

A function needs a return annotation, `-> None` when it returns nothing:

<!-- typy-expect: rejected E0204 at 1:5 -->
```python
def show(value: int):
    print(value)
```
