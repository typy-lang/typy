# Functions

## What TyPy accepts

| Item | Rule |
| --- | --- |
| Function definitions at the top of the module, with annotated parameters and return | FUN-01, TYP-01 |
| Recursion; running out of stack ends the program with `RecursionError` | FUN-02 |
| Positional, keyword and default arguments, defaults evaluated once | EXP-16 |
| Positional-only parameters before `/` and keyword-only parameters after `*`, bound as CPython binds them; a keyword for a positional-only parameter, or a positional argument past the others, is E0404 | FUN-03 |
| `*args: T` and `**kwargs: T`, which take the arguments left over as a `tuple[T, ...]` and a `dict[str, T]` | EXP-17 |
| `f(*iterable)` and `f(**dict)` in a call of the program's code, and `print(*iterable)`, bound as CPython binds them | EXP-17 |
| A function of the program as a value of type `Callable[[A, ...], R]`: stored, passed, returned, called, compared and hashed | FUN-05, NFR-19 |
| A method read through an instance as a bound method, and through its class as a function | FUN-05 |
| A function defined inside a function, which reads the variables of the functions around it | FUN-04 |
| A generator function, whose `yield` statements give its items one step at a time | SMT-11, FUN-07 |
| `send`, `throw` and `close` of a `Generator[Y, S, R]`, and `received = yield value` | SMT-11, FUN-07 |
| `yield from` any iterable, and `return value` of a `Generator[Y, S, R]` | SMT-11, FUN-07 |
| A generator freed while suspended closes, running its `finally` blocks | FUN-11 |

A function value is the `key` of `sorted`, `list.sort`, `min` and `max`;
see [Ordering by a key](../library/functions-and-modules.md#ordering-by-a-key).
A built-in function is a value where a `Callable` is expected, such as
`sorted(words, key=len)` or `measure: Callable[[str], int] = len`, and is
written `<built-in function len>` (FUN-08). So is a method of a built-in
type read through the type, such as `sorted(words, key=str.lower)` or
`add: Callable[[list[int], int], None] = list.append`, written
`<method 'lower' of 'str' objects>`, a method read through a built-in
value, such as `add: Callable[[str], None] = words.append` or
`key="-".join`, written `<built-in method append of list object at
0x...>`, and a function of `math` or `sys`, such as `key=math.fabs`. A
built-in type is a function of its arguments there too, such as
`sorted(numbers, key=str)` or `parse: Callable[[str], int] = int`, written
`<class 'int'>` (FUN-14). With
no `Callable` expected, `chr` and `ord` have their one type, and any other
built-in function or method is E0203, since its arguments may be of
several types.

A nested function inside a generic function, or inside a function with a
protocol parameter, is checked and made in each copy of the function, and
keeps Python's `__qualname__`, such as `pair_with.<locals>.make` (FUN-10).

A method read through a protocol value, such as `area = shape.area`, is
a bound method of the method of the object's class, written, compared and
hashed as that method (FUN-09). So is a method read through an instance
whose subclass defines it again as a static method, or overrides it with a
parameter or a result held otherwise, such as `int | None` for `int`: the
value has the type of the method read, and calls the object's own method.

A nested function, a method or a `lambda` with defaults, `*args` or
`**kwargs` is a value where a `Callable` is expected that its parameters
take; see [Variable numbers of arguments](#variable-numbers-of-arguments)
(FUN-19). A nested function may have type parameters of its own; see
[Nested generic functions](generics.md#nested-generic-functions) (FUN-18).
A class of the program as a `Callable` is planned for 0.14 (FUN-16).

## Decorators

A decorator on a function at the top of a module or nested in a function,
also inside an `if`, a loop, a `with`, a `try` or a `match` there, is any
expression that takes the function, as CPython allows (FUN-06).
The decorators are evaluated from the top down and applied from the bottom
up when the `def` runs, and the name holds what the outermost returns, so
a read of the name, also inside the function, reads that value. A
decorator may be generic, such as one that takes a `Callable[[T], T]`.

<!-- typy-expect: accepted -->
```python
from collections.abc import Callable


def trace(function: Callable[[int], int]) -> Callable[[int], int]:
    def wrapper(value: int) -> int:
        print("calling with", value)
        return function(value)

    return wrapper


@trace
def increment(value: int) -> int:
    return value + 1


print(increment(2))
```

The name holds a `Callable` value, which names no parameters, so a call
through it passes its arguments by position: `increment(value=2)` is
E0404. A generic function cannot be decorated, since it is no one value
(E0202). Decorators of methods are planned for 0.14 (FUN-15), and class
decorators other than `@dataclass` for 1.0 (CLS-38).

## Variable numbers of arguments

A parameter written `*args: T` takes the positional arguments left after
the other parameters as a `tuple[T, ...]`, and one written `**kwargs: T`
takes the keyword arguments no other parameter names as a
`dict[str, T]`, in the order the call gives them. The annotation is the
type of each argument, as in `typing`. A call with nothing left for them
gives an empty tuple and an empty dict:

<!-- typy-expect: accepted -->
```python
def describe(name: str, *tags: str, sep: str = ", ", **sizes: int) -> str:
    parts = [name, sep.join(tags)]
    for key in sizes:
        parts.append(f"{key}={sizes[key]}")
    return " | ".join(parts)


print(describe("box"))
print(describe("box", "red", "new", sep="/", width=2, height=3))
```

A call evaluates every argument in source order before it binds them, as
CPython does, so a keyword that `**kwargs` takes and one a named
parameter takes run in the order they are written. A keyword that names
a positional-only parameter goes to `**kwargs`, and so does one that
names `*args` or `**kwargs` itself.

Methods, `__init__`, nested functions and generic functions take them
too; a type parameter is solved from the arguments `*args` takes, as in
`def first[T](*items: T) -> T`. An override keeps the `*args` and
`**kwargs` of the method it overrides, E0404 (CLS-04). A function of the
module with defaults, `*args` or `**kwargs` is a value where a `Callable`
is expected that its parameters take, such as `apply(total, 2, 3)` or
`twice: Callable[[int], int] = scale` with `by: int = 2`; it is called
with the `Callable`'s arguments by position, and stays the same function
to `is` and `==` (FUN-13). Read where nothing is expected, it is E0203.

So is a nested function, a method read through an instance, a class or a
protocol value, and a `lambda` (FUN-19). A keyword-only parameter with a
default takes its default. The value is the function itself to `is`,
`==`, `hash` and `repr`: `obj.scale` is still a bound method of the
object's own class. In a `lambda`, `*args` holds the arguments past the
other parameters, which have one type, and a parameter past them takes
its default:

<!-- typy-expect: accepted -->
```python
from collections.abc import Callable


def apply(function: Callable[[int], int], value: int) -> int:
    return function(value)


class Meter:
    def scale(self, value: int, extra: int = 0) -> int:
        return value * 2 + extra


def outer(base: int) -> int:
    def add(value: int, *more: int) -> int:
        return value + base + sum(more)

    by_three = apply(lambda value, by=3: value * by, 4)
    return apply(add, 1) + apply(Meter().scale, 2) + by_three


print(outer(10))
```

A call may also unpack its arguments; see
[Unpacking in a call](#unpacking-in-a-call).

## Unpacking in a call

A call of a function, method or class of the program, or of a method through a
protocol (EXP-22), passes each value of an iterable written `*iterable` as a
positional argument, and each entry of a `dict[str, T]` written `**dict` as a
keyword argument, as CPython does. `print(*values)` writes each value:

<!-- typy-expect: accepted -->
```python
def describe(name: str, count: int = 1, *, unit: str = "item") -> str:
    return f"{count} {unit}(s) of {name}"


def total(*numbers: int) -> int:
    return sum(numbers)


def forward(*numbers: int) -> int:
    return total(*numbers)


order = ("apple", 3)
options = {"unit": "box"}
values = [1, 2, 3]
print(describe(*order, **options))
print(forward(*values), *values, sep=", ")
```

The items of a tuple of a fixed number of items bind as positional
arguments would, and are checked as they are. The values of any other
iterable, and the entries of a dict, bind when the call runs, and a call
that does not fit raises CPython's `TypeError` with CPython 3.15's
message, such as `pair() missing 1 required positional argument: 'b'`
(ADR-0073). A call evaluates every positional argument, each `*iterable`
read to its end, before its keyword arguments, as CPython does.

From the first `*iterable` of unknown length on, the positional values
have one type: the one the parameters they may reach share, which each
value must fit, E0404, or else the first value's, which the others must
have, E0409, and which must fit each of those parameters. The values of
the `**` dicts
have one type too, which must fit each parameter a keyword may still
fill and the values of `**kwargs`.

A call of a builtin, a function value or an exception class takes `*` of a tuple
display or of a variable that holds a fixed tuple, and `**` of a dict display
whose keys are `str` literals, each written out as the values it gives (EXP-19):

<!-- typy-expect: accepted -->
```python
point = (7, 3)
print(max(*point), min(*(4, 9, 2)), *point, **{"sep": "-"})
```

A call of a function value takes `*` of any iterable (EXP-22). The
values before the first `*` bind to the first parameters, and the values
from it on fill the rest, which must take one type. When the program
runs, the call checks that they are as many as the `Callable` takes, and
raises CPython's `TypeError` otherwise, naming the function the value
holds, such as `total() takes 2 positional arguments but 3 were given`;
for too few values the message does not name the missing parameters,
which a `Callable` type does not carry:

<!-- typy-expect: accepted -->
```python
from collections.abc import Callable


def total(first: int, second: int) -> int:
    return first + second


add: Callable[[int, int], int] = total
values = [1, 2]
print(add(*values), add(1, *values[1:]))
```

`**` in a call of a function value binds by the names of the function it holds,
which a `Callable` type does not carry, and is planned for 1.0 (EXP-21). A call
of an exception class takes `*` of any iterable and `**` of a dict held in a
variable; see [The arguments and the exit
code](../library/exceptions.md#the-arguments-and-the-exit-code). The same
unpacking in a call of a builtin other than `print` is planned for 0.11
(EXP-22).

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

### Delegating with `yield from`

`yield from source` yields every item of `source`, any iterable, and
passes on to a generator source what resumes your generator: a value
`send` gives, an exception `throw` gives, and `close()`. A generator
annotated `Generator[Y, S, R]` returns a value of type `R` with
`return value`; `next()` then raises `StopIteration(value)`, and
`result = yield from source` gets the value:

<!-- typy-expect: accepted -->
```python
from collections.abc import Generator


def averaged() -> Generator[None, float, float]:
    total = 0.0
    count = 0
    while count < 3:
        value = yield
        if value is not None:
            total += value
            count += 1
    return total / count


def report() -> Generator[None, float, None]:
    average = yield from averaged()
    print("average", average)


reader = report()
next(reader)
for value in [1.0, 2.0, 6.0]:
    try:
        reader.send(value)
    except StopIteration:
        print("done")
```

The source's items must fit the `Y` your generator yields, and a
generator source must receive what yours is sent: the same `S`, or one of
them `S | None` of the other. A generator that returns nothing gives
`None`, and so does `yield from` over any other iterable. A value sent on
to an iterator that is not a generator raises `AttributeError`, as in
CPython 3.12 (ADR-0072).

### Closing a generator

A generator you stop using before it ends still runs its `finally`
blocks and the `__exit__` of its `with` statements. When its last
reference goes while it is suspended, TyPy closes it, as CPython does:
`GeneratorExit` is raised at its `yield`. A loop that leaves a generator
early closes it before the statement after the loop:

<!-- typy-expect: accepted -->
```python
from collections.abc import Iterator


def numbers() -> Iterator[int]:
    try:
        yield 1
        yield 2
    finally:
        print("cleaned up")


for number in numbers():
    print(number)
    break
print("after the loop")
```

This prints `1`, `cleaned up`, then `after the loop`. A generator in a
list, a dict, a tuple or an attribute closes when the container goes, in
the order CPython closes them, and one never started runs nothing. A
generator in a variable of a function that an exception leaves lives as
long as the exception, as CPython's traceback keeps the function's
variables: it closes when the `except` clause that handles the exception
ends, or after the report of one nothing catches (ADR-0070). A generator
still suspended in a module variable when the program ends closes then,
after everything else the program printed: those of the main module
first, in the order its code binds them, then those of the modules it
imports (ADR-0071). An
exception raised while a freed generator closes cannot reach your code:
it is written to standard error after `Exception ignored in: <generator
object NAME at 0x...>`, and the program goes on (FUN-11, ADR-0069).

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
| A function or a method with a keyword-only parameter read where no `Callable` is expected, since a `Callable` passes every argument by position | E0202 |
| A function or a method whose keyword-only parameter has no default, where a `Callable` is expected | E0404 |
| A method with type parameters of its own as a value | E0202 |
| A function of the module with `*args` or `**kwargs` read where no `Callable` is expected | E0203 (FUN-13) |
| A nested function or a method with `*args` or `**kwargs` read where no `Callable` is expected | E0203 (FUN-19) |
| A `lambda` with `**kwargs`, which a `Callable` never fills, or with `*args` where the `Callable` passes nothing past the other parameters | E0203 (FUN-19) |
| `*args` of a `lambda` where the `Callable` passes values of two types past the other parameters | E0409 (FUN-19) |
| `*` of a list or another iterable, or `**` of a dict held in a variable, in a call of a builtin other than `print`, planned for 0.11 | E0201 (EXP-22) |
| `*` of a value only the run knows in a call of `OSError` or a class derived from it, whose `errno`, `strerror` and file names are read by position | E0202 (EXP-22) |
| `*` in a call of an exception class whose positional values have more than one type | E0409 (EXP-22) |
| `**` in a call of `print` of a value that is no `dict[str, V]`, or of dicts of different value types | E0404 (EXP-22) |
| `**` in a call of a function value, planned for 1.0 | E0201 (EXP-21) |
| `*` in a call of a function value whose values may reach parameters of more than one type | E0404 (EXP-22) |
| An assignment to a method of an instance, such as `box.close = other`, since TyPy does not replace a class's attributes when the program runs | E0414 |
| A property read through its class as a value, such as `Box.v`, which is a `property` object, planned for 1.0 | E0201 (FUN-17) |
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
