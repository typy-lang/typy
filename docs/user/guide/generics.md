# Generics

A generic function or class takes a type parameter list, as PEP 695
writes it: `def first[T](items: list[T]) -> T` or `class Box[T]:`. TyPy
compiles one copy of the function, or of the class, for each set of types
the program uses it with, so a generic call costs what a call written for
those types costs (GEN-01 to GEN-03, ADR-0023, ADR-0028).

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `def name[T, U](...)`: a generic function at module level | GEN-01 |
| `def name[T](self, ...)`: a method with type parameters, also a static method | GEN-01 |
| `class Name(Protocol):`, satisfied by any class with its members | GEN-04 |
| `class Name[T]:`, a generic class, and `Name[int]` in annotations and calls | GEN-01 |
| `class Name[T](Base):`, a generic class with a base of the program | GEN-08 |
| `class Sub(Name[int]):` and `class Sub[T](Name[T]):`, a class derived from a copy | GEN-08 |
| `[T: Bound]`, a bound, and `[T: (A, B)]`, constraints | GEN-02 |
| One compiled copy for each set of type arguments, with no type at run time | GEN-03 |
| A generic function or class of another module, by name or through the module | GEN-07 |

## Generic functions

A call gives the type parameters their types from its arguments, as type
checkers do. Two arguments that give one parameter different types give
it their union:

<!-- typy-expect: accepted -->
```python
def first[T](items: list[T]) -> T:
    return items[0]


def pair[K, V](key: K, value: V) -> tuple[K, V]:
    return (key, value)


def repeat[T](value: T, times: int) -> list[T]:
    result: list[T] = []
    for _ in range(times):
        result.append(value)
    return result


print(first([1, 2]), first(["a"]), pair("a", 1), repeat("x", 3))
```

The body is checked once, before any call, with each type parameter as a
type of its own that offers only what every type offers: it can be
passed, returned, stored, compared with `==` and printed. A method or an
operator on it is E0412 or E0401, since some type it may stand for has
none:

<!-- typy-expect: rejected E0401 at 2:12 -->
```python
def twice[T](value: T) -> T:
    return value + value
```

A parameter that no argument gives, such as the `T` of
`def empty[T]() -> list[T]`, is E0404.

## Bounds and constraints

A bound, `[S: Shape]`, makes the parameter offer what its bound offers,
and each call checks that its argument is one. Constraints,
`[N: (int, float)]`, make the parameter one of the listed types: the body
is checked once for each, and a call gives the first that takes its
argument:

<!-- typy-expect: accepted -->
```python
class Shape:
    def area(self) -> float:
        return 1.0


class Square(Shape):
    def __init__(self, side: float) -> None:
        self.side = side

    def area(self) -> float:
        return self.side * self.side


def largest[S: Shape](shapes: list[S]) -> S:
    best = shapes[0]
    for shape in shapes:
        if shape.area() > best.area():
            best = shape
    return best


def doubled[N: (int, float)](value: N) -> N:
    return value + value


print(largest([Square(1.0), Square(3.0)]).side, doubled(3), doubled(1.5))
```

A call whose argument is outside the bound or the constraints is E0404.
A function that calls itself with a larger type each time, as
`f(x)` calling `f([x])`, would need copies without end; past 64 copies
inside one another it is E0428.

## Generic classes

A generic class's type parameters type its attributes and methods. A call
takes the types from the arguments of `__init__`, as a generic function's
call does, or from the class's type arguments, `Box[str]("a")`. An
annotation names the class with its type arguments, `Box[int]`; the bare
name is E0408 outside the class's own body:

<!-- typy-expect: accepted -->
```python
class Node[T]:
    def __init__(self, value: T, next: "Node[T] | None") -> None:
        self.value = value
        self.next = next

    def values(self) -> list[T]:
        found: list[T] = [self.value]
        rest = self.next
        if rest is not None:
            found.extend(rest.values())
        return found


def first[T](node: Node[T]) -> T:
    return node.value


chain = Node(1, Node(2, None))
words = Node[str]("a", None)
print(chain.values(), first(words))
print(type(chain) is Node, isinstance(words, Node))
```

Each copy, such as `Node[int]`, is a class of its own, which holds an
`int` in place, and it answers as the generic class: `type(chain)` is
`Node`, `isinstance(words, Node)` holds and `__name__` is `"Node"`, as in
CPython. A generic dataclass gets its methods for each copy.
A class variable or a default the class body gives belongs to the
generic class, so every copy shares it, as every instance shares it in
CPython: `Counter.made += 1` in `Counter[int]` and in `Counter[str]`
counts in one variable. Its annotation may not name a type parameter,
since one value cannot have each copy's type; declare such an attribute
without a value and assign it in `__init__`.
A static method that names none of the class's type parameters is the
same for every copy and is compiled once. Another static or class
method called through the class takes the types from its arguments,
`Box.wrap("a")`, or from the class's type arguments, `Box[int].make(3)`;
when neither gives them, the call is E0404.
`isinstance(x, Node[int])` is refused, as CPython raises `TypeError` for
it.

## Generic classes with a base

A generic class may derive from a class of the program, and every copy
then derives from it: a `Box[int]` is a `Base`, takes `Base`'s methods
and may be passed where a `Base` is expected (GEN-08, ADR-0040):

<!-- typy-expect: accepted -->
```python
class Base:
    def __init__(self, name: str) -> None:
        self.name = name

    def describe(self) -> str:
        return self.name + " holds " + self.held()

    def held(self) -> str:
        return "nothing"


class Box[T](Base):
    def __init__(self, item: T) -> None:
        super().__init__("box")
        self.item = item

    def held(self) -> str:
        return str(self.item)


def show(thing: Base) -> str:
    return thing.describe()


print(show(Box(1)), show(Box("a")), isinstance(Box(2.5), Base))
```

A method that overrides one of the base is checked with the type
parameters as they are: `def held(self) -> T` would not fit a base
method that returns `str`, E0404, as type checkers report.
`isinstance(x, Box)` on a variable whose type is `Base` keeps that type,
since which copy the object belongs to is known only when the program
runs; narrowing it is planned for 1.0 (GEN-14), and so are the
comparison methods of a generic dataclass derived from a dataclass,
which need it. Pass `eq=False` to such a dataclass, or derive it from a
class that is not a dataclass.

## Classes derived from a generic class

A class may derive from a copy of a generic class, which names its type
arguments: `class IntBox(Box[int])` holds an `int` in place and is a
`Box[int]`. A generic class may derive from a copy for its own type
parameters, `class Pair[T](Box[T])`, and each of its copies then derives
from the matching copy, `Pair[str]` from `Box[str]` (GEN-08, ADR-0040):

<!-- typy-expect: accepted -->
```python
class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item

    def get(self) -> T:
        return self.item


class IntBox(Box[int]):
    def get(self) -> int:
        return super().get() * 10


class Pair[T](Box[T]):
    def __init__(self, item: T, other: T) -> None:
        super().__init__(item)
        self.other = other

    def both(self) -> tuple[T, T]:
        return (self.get(), self.other)


def read(box: Box[int]) -> int:
    return box.get()


print(read(IntBox(3)), read(Pair(1, 2)), Pair("a", "b").both())
print(isinstance(IntBox(1), Box), type(Pair(1, 2)) is Pair)
```

An override takes the copy's types: `def get(self) -> str` in `IntBox`
is E0404, since `Box[int].get` returns an `int`. The bare name of a
generic class as a base, `class Sub(Box)`, is E0408, as in annotations.
A class derived from a copy of a generic class that has class methods,
and a dataclass derived from a copy of a generic dataclass, are planned
for 1.0 (GEN-15).

## Generic methods

A method of a class that is not generic may take type parameters of its
own. A call gives them their types from the arguments after `self`, as
for a function, and runs the copy for those types:

<!-- typy-expect: accepted -->
```python
class Store:
    def __init__(self, name: str) -> None:
        self.name = name

    def pair[T](self, value: T) -> tuple[str, T]:
        return (self.name, value)

    @staticmethod
    def wrap[T](value: T) -> list[T]:
        return [value]


store = Store("main")
print(store.pair(1), store.pair("a"), Store.wrap(2.5))
```

A generic method may override one with as many type parameters, and a
call through the base runs the copy of the object's class's method for
the same type arguments, as CPython runs the override (GEN-09):

<!-- typy-expect: accepted -->
```python
class Visitor:
    def visit[T](self, value: T) -> list[T]:
        return [value]


class Doubler(Visitor):
    def visit[T](self, value: T) -> list[T]:
        return super().visit(value) + [value]


def run(visitor: Visitor) -> None:
    print(visitor.visit(1), visitor.visit("a"))


run(Visitor())
run(Doubler())
```

A generic method that overrides a method without type parameters, or
one with another number of them, or one in a generic class, is planned
for 1.0 (GEN-16).

A method of a generic class may take type parameters of its own. A call
takes the class's from the instance's copy and solves the method's own
from the arguments: `Box(1).paired("a")` runs the copy of `paired` for
`int` and `str`. A static method that names none of the class's type
parameters is generic over its own alone, so `Box.both(1, 2)` needs no
type argument of `Box` (GEN-09).

A class method may take type parameters of its own, and so may one
whose parameter names a protocol: `Registry.make("r", 1.5)` runs the
copy for a `float`, and a class derived from `Registry` gets copies of
its own, as for any class method (GEN-09). It is called through its
class; through an instance it is planned for 1.0 (GEN-16).

## Generics of other modules

A generic function or class of another module is used as one of the
module's own: imported by name, `from tools import first, Box`, or
through the module, `tools.first([1])` and `tools.Box[int]`. Its copies
are made from its source and read the names of the module that defines
it, so a copy may call that module's functions, including those whose
names start with an underscore, and read its variables (GEN-07). A type
argument outside a bound or the constraints is reported at the call, in
the module that makes it.

## Protocols

A protocol lists the methods and attributes a class needs, and any class
that has them satisfies it, without naming it. A parameter whose type is
a protocol stands for each such class: TyPy compiles the function once
for each class it is called with, as if it had a type parameter bounded
by the protocol:

<!-- typy-expect: accepted -->
```python
from typing import Protocol


class Shape(Protocol):
    sides: int

    def area(self) -> float: ...


class Square:
    def __init__(self, side: float) -> None:
        self.side = side
        self.sides = 4

    def area(self) -> float:
        return self.side * self.side


class Triangle:
    def __init__(self) -> None:
        self.sides = 3

    def area(self) -> float:
        return 0.5


def describe(shape: Shape) -> str:
    return f"{shape.sides} sides, area {shape.area()}"


def total(shapes: list[Shape]) -> float:
    return sum([shape.area() for shape in shapes], 0.0)


mixed: list[Square | Triangle] = [Square(2.0), Triangle()]
print(describe(Square(1.0)), describe(Triangle()), total(mixed))
```

A class satisfies a protocol when it has each method, taking the same
parameters by name with types that accept the protocol's, a default
where the protocol's method has one, and then only parameters with
defaults, and returning a type the protocol's accepts; and each
attribute with the same type. A call with a class that falls short is
E0404, and the message names the member. A protocol's methods are never
run, so their bodies are usually `...`.

A protocol has no instances: calling it is E0401, and so is
`isinstance` with a protocol, which raises `TypeError` in CPython
(GEN-05).

## Refused and planned

`TypeVar` and `Generic` from `typing` are E0202: a type parameter list
replaces them (GEN-01). A generic type alias, `type Pair[T] = tuple[T,
T]`, is described with the other [type aliases](types.md#type-aliases).
Type parameters of a special method or a property, of a method that
overrides or is overridden by one with other type parameters, and a
class method with type parameters called through an instance, are
planned for 1.0 (GEN-16). A protocol as the type of a
variable, an attribute, a result or a container item outside a
parameter, a class that names a protocol as its base, and properties,
static methods, class methods and class variables of a protocol are
planned for 0.5 (GEN-10). A class variable or a default whose
annotation names a type parameter of its class (GEN-11), a generic
exception class (GEN-12), a generic protocol (GEN-13), class methods and
dataclass fields inherited through a copy (GEN-15), and `*Ts`, `**P` and
defaults of type parameters (GEN-06) are planned for 1.0.
