# Classes

Classes are standard Python classes. Every attribute is declared in the
class body or assigned in `__init__`, and has one type.

## What TyPy accepts

| Item | Rule |
| --- | --- |
| Classes with `__init__`, methods and attributes declared in the body or assigned in `__init__` | CLS-01 |
| Assigning or calling a name the class does not have is refused | CLS-02 |
| A class derived from one class of the program, with its attributes and methods, overriding them | CLS-04 |
| Class variables and attribute defaults given a value in the class body | CLS-11 |
| `@staticmethod`, `@classmethod`, and `@property` with its setter | CLS-06 |
| `typing.Self` in an instance or a class method's parameters and result; see [The class a method is called through](#the-class-a-method-is-called-through) | TYP-30 |
| `__eq__`, `__ne__`, `__lt__`, `__le__`, `__gt__`, `__ge__` and `__hash__` | CLS-07 |
| `__add__`, `__radd__` and the other arithmetic, bitwise and unary methods | CLS-07 |
| `NotImplemented`, returned by an operator or comparison method to give way | CLS-21 |
| `__len__`, `__getitem__`, `__setitem__`, `__delitem__`, `__contains__` and `__bool__` | CLS-07 |
| `@dataclass`, with `eq`, `order`, `frozen`, `match_args` and `field(default=...)` | CLS-05, CST-06 |
| A class derived from `typing.NamedTuple`, whose instances are tuples of its fields | CLS-27 |
| A class derived from `abc.ABC`, with `@abstractmethod`; see [Abstract classes](#abstract-classes) | CLS-45 |
| A class derived from `enum.Enum`, `IntEnum`, `StrEnum`, `Flag` or `IntFlag`, with its members; see [Enum classes](#enum-classes) | CLS-44 |
| `__str__` and `__repr__`, used by `print`, `str`, `repr`, f-strings and containers | CLS-08 |
| A type object for every type, with `object` as the root | CLS-10 |
| `type(x)`, class and built-in type names as values, `__name__` | CLS-12 |

Inside `__init__`, `self` is only used to assign and read attributes until
every attribute holds a value, so no attribute is ever read empty. A
signature that names its own class, or a class defined below it, quotes
the name, such as `"Node | None"`.

Instances are equal only to themselves, hashed by identity and true
unless their class defines the methods below (CLS-07).
A class may derive from an exception class, as
[Exceptions](../library/exceptions.md) describes (CLS-03).

## Example

<!-- typy-expect: accepted -->
```python
class Account:
    owner: str

    def __init__(self, owner: str, balance: int = 0) -> None:
        self.owner = owner
        self.balance = balance
        self.history: list[int] = []

    def deposit(self, amount: int) -> None:
        self.balance += amount
        self.history.append(amount)

    def __repr__(self) -> str:
        return f"Account({self.owner!r}, {self.balance})"


class Node:
    def __init__(self, value: int, next: "Node | None" = None) -> None:
        self.value = value
        self.next = next


account = Account("ada")
account.deposit(50)
chain = Node(1, Node(2))
print(account, account.history, [account], type(account).__name__)
print(chain.value, chain.next is not None, type(chain) is Node)
```

## Inheritance

A class may derive from one class of the program, named by its name or
as `module.Class` (CLS-04). It holds its base's attributes and methods,
and its instances may be used wherever the base is expected: as an
argument, in a variable, in a `list[Base]` or as a `Base | None`.

<!-- typy-expect: accepted -->
```python
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def describe(self) -> str:
        return "a shape named " + self.name

    def __repr__(self) -> str:
        return f"Shape({self.name!r})"


class Square(Shape):
    def __init__(self, side: int) -> None:
        self.name = "square"
        self.side = side


class Label(Shape):
    pass


shapes: list[Shape] = [Square(2), Label("tag")]
for shape in shapes:
    print(shape.describe(), type(shape).__name__, shape)
```

A subclass may define a method of its base again. A call runs the
method of the instance's own class, whatever the type of the variable
that holds it, as in CPython:

<!-- typy-expect: accepted -->
```python
class Shape:
    def area(self) -> float:
        return 0.0

    def describe(self) -> str:
        return f"area {self.area()}"


class Square(Shape):
    def __init__(self, side: float) -> None:
        self.side = side

    def area(self) -> float:
        return self.side * self.side


shapes: list[Shape] = [Shape(), Square(3.0)]
print([shape.describe() for shape in shapes])
```

An override takes the base method's parameters in the same order and
with the same names. Each parameter accepts the type the base's takes,
by position and by keyword wherever the base's does, since CPython gives
the override the arguments a call through the base passes, and the
result is one the base's result type accepts, as type checkers
require; a mismatch is E0404. A parameter with a default in the base
has one in the override, which may differ: a call that leaves it out
takes the default of the object's class, as in CPython. An override may
take or return a type held differently from the base's, such as
`int | None` where the base has `int`; with a default of its own for
such a parameter it is planned for 0.14 (CLS-34).
`__init__` is not compared, since it runs only on the class being made.

<!-- typy-expect: rejected E0404 at 7:9 -->
```python
class Shape:
    def area(self) -> float:
        return 0.0


class Label(Shape):
    def area(self) -> str:
        return "none"
```

A subclass may declare an attribute of its base again only with the
same type. The base of a class is `object`, an exception class or a
class of the program defined before it; a built-in type such as `int` or
`list` is E0201, and several bases are planned for after 1.0.

A value of type `type[C]` holds `C` or a class derived from it, as in
CPython: a `type[Square]` passes where a `type[Shape]` is expected, and
`type(x)` of an instance whose static class is `Shape` is a `type[Shape]`
that holds the class the instance has when the program runs, which
`print`, `__name__`, `is`, `==`, `isinstance` and `issubclass` read (CLS-18,
ADR-0084). When a class of the program derives from `C`, calling such a
value, or reading a class variable or a class method through it, is
planned for 0.14 (CLS-39); call the class by its name instead. `cls` in a
class method holds its own class, and is called as it is.

<!-- typy-expect: rejected E0201 at 11:12 -->
```python
class Shape:
    pass


class Circle(Shape):
    pass


def copy(shape: Shape) -> Shape:
    kind = type(shape)
    return kind()
```

## Calling the base with `super()`

In a method, `super().name(...)` calls the method of the nearest base
that defines it, and `super(Class, self)` naming the enclosing class does
the same (CLS-14). In `__init__`, `super().__init__(...)` runs the base's
`__init__`, which assigns the inherited attributes:

<!-- typy-expect: accepted -->
```python
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def describe(self) -> str:
        return "shape " + self.name


class Square(Shape):
    def __init__(self, side: float) -> None:
        super().__init__("square")
        self.side = side

    def describe(self) -> str:
        return super().describe() + f" with side {self.side}"


print(Square(2.0).describe())
```

When the base's `__init__` uses `self` other than through its
attributes, as by calling a method, it may run an override that reads
an attribute of the subclass. Such a call of `super().__init__(...)`
comes after the subclass assigns its own attributes, E0309 otherwise,
so no attribute is read before it holds a value. `super()` outside a
method is E0414. `super(Base, self)` names the enclosing class or one of
its bases, and the method found is that of the nearest base of `Base`, as
in CPython (CLS-26); with an instance other than `self` it is planned for
0.14 (CLS-29). `super()` binds the instance before the arguments are
evaluated, and a class method called through it takes the class of the
instance as `cls` (CLS-25). Where the enclosing class or a class derived
from it defines that class method again, the call is planned for 0.14
(CLS-46).

A method may also be called through a class with the instance first,
`Shape.describe(square)`, which runs the method that class holds or
inherits, not an override, as in CPython. In `__init__`,
`Shape.__init__(self, "square")` on the instance being built does what
`super().__init__("square")` does, for any base the class derives from,
and its attributes count as assigned. A property called through its
class is E0401, since CPython's property object cannot be called
(CLS-06).

An instance never holds an attribute without a value: calling a class
whose body declares an attribute that no `__init__` that runs assigns,
such as a subclass without `__init__` that declares one of its own, is
E0309.

<!-- typy-expect: rejected E0201 at 1:13 -->
```python
class Count(int):
    pass
```

## Testing the class with `isinstance`

`isinstance(value, Class)` tests whether a value is an instance of a
class or of one derived from it, and a tuple of classes tests each
(CLS-09). The classes are named in the call: classes of the program,
built-in types such as `int` or `str`, and exception classes. Where a
test holds, a variable has the tested class's type, so its attributes
and methods can be used; the narrowing ends when the variable is
assigned (TYP-07):

<!-- typy-expect: accepted -->
```python
class Shape:
    pass


class Circle(Shape):
    def __init__(self, radius: float) -> None:
        self.radius = radius


def radius(shape: Shape | None) -> float:
    if isinstance(shape, Circle):
        return shape.radius
    return 0.0


print(radius(Circle(2.0)), radius(Shape()), radius(None))
```

A test also narrows after an `if` whose body cannot fall through, such
as `if not isinstance(shape, Circle): return`, and in the right operand
of `and`. When the checker can tell the answer, as for
`isinstance(3, int)`, nothing is tested when the program runs; `bool`
counts as `int`, as in CPython. A class held in a value, such as
`isinstance(shape, kind)` with `kind = type(other)` or a parameter of type
`type[Square]`, is tested when the program runs, and narrows the value to
the class its type names (CLS-18). `issubclass(class, classes)` tests a
class against a class or a tuple of them, named or held in a value, as
CPython answers: `issubclass(bool, int)` is `True` (CLS-19). Either test
takes a tuple of classes held in a value, such as `kinds = (int, float)`
or a `tuple[type[Shape], ...]`, and tests each class in order. A value
that is no class, or a tuple with one, is E0401, as CPython raises
`TypeError`. A class pattern may name a class held in a variable too, as
[Compound statements](compound-statements.md#class-patterns) shows.

## Class variables and defaults

A value in a class body is read as type checkers read it (CLS-11):

| Written as | Meaning |
| --- | --- |
| `count = 0` or `count: ClassVar[int] = 0` | A class variable: one value the class holds, read as `Counter.count` or `self.count` |
| `step: int = 1` | An attribute with a default: each new instance starts with the class's value, then `__init__` may assign it |

<!-- typy-expect: accepted -->
```python
from typing import ClassVar


class Counter:
    created = 0
    label: ClassVar[str] = "counter"
    step: int = 1

    def __init__(self, start: int) -> None:
        self.value = start
        Counter.created += 1

    def advance(self) -> int:
        self.value += self.step
        return self.value


class Fast(Counter):
    step: int = 5


counter = Counter(0)
fast = Fast(10)
print(counter.advance(), fast.advance(), Counter.created, fast.label)
```

The class body's values are computed where the `class` statement runs,
as in CPython, and a mutable default is one object that every instance
shares. A class variable is assigned through its class, as
`Counter.created += 1`; assigning it through an instance, which gives
that one instance a value of its own in CPython, is E0414, and so is
assigning a default through the class.

## Static methods and properties

`@staticmethod` makes a function of the class that takes no instance,
called as `Class.name(...)` or through an instance. `@property` makes a
method that is read as an attribute, and `@name.setter` the method that
assigning the attribute calls (CLS-06):

<!-- typy-expect: accepted -->
```python
class Temperature:
    def __init__(self, celsius: float) -> None:
        self._celsius = celsius

    @staticmethod
    def from_fahrenheit(fahrenheit: float) -> "Temperature":
        return Temperature((fahrenheit - 32.0) * 5.0 / 9.0)

    @property
    def celsius(self) -> float:
        return self._celsius

    @celsius.setter
    def celsius(self, value: float) -> None:
        if value < -273.15:
            raise ValueError("below absolute zero")
        self._celsius = value


boiling = Temperature.from_fahrenheit(212.0)
boiling.celsius = boiling.celsius - 1.0
print(boiling.celsius)
```

A subclass may define a property again, and reading it runs the
instance's class's getter. A getter takes only `self`; a setter takes
`self` and a value of the getter's type and returns `None`. Assigning a
property without a setter is E0414, as CPython raises `AttributeError`
for it. A method without `self` is E0404; decorate it with
`@staticmethod`. A property is assigned through any expression, such
as `make().name = value` or `item.name += 1`, the value evaluated
first, as in CPython. A subclass may redefine a static method, or a
method as a static one, and a call through an instance runs the one of
the object's class, as in CPython; a call through a class runs that
class's (CLS-06). `item.name(arguments)` for a property calls the value
its getter gives, a function value, as CPython does; a value of another
type is E0401 (FUN-05).

## Class methods

`@classmethod` makes a method that takes the class it is called
through, `cls`, as CPython does: `Square.unit()` passes `Square`, so
`cls(...)` makes a `Square`. TyPy compiles a copy of the method for
each class that has it, and checks `cls(...)` against that class's
`__init__` (CLS-06):

<!-- typy-expect: accepted -->
```python
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    @classmethod
    def unit(cls) -> "Shape":
        return cls("unit " + cls.__name__)


class Square(Shape):
    pass


print(Shape.unit().name, Square.unit().name, type(Square.unit()).__name__)
```

A static method is called through its class, or through any expression
that gives the class or one of its instances, which is evaluated first,
as in CPython. A class method is called through its class, also a
subclass defined in another module, which gets a copy of its own, and
through an instance, which passes the instance's class as `cls`
(CLS-25). Through an instance of a generic class it is planned for 0.14
(CLS-30). Assigning a class variable through a subclass that inherits
it gives the subclass a variable of its own, as in CPython: the
subclass, its subclasses and their instances read it from then on, and
the base keeps its own (CLS-25). Assigning it through a variable that
holds such a subclass, such as `cls` in a class method, is planned for
0.14 (CLS-31).

## The class a method is called through

`Self` of `typing`, in the parameters and the result of an instance or a
class method, is the class the method is called through (TYP-30,
ADR-0113). A method inherited by a derived class returns that class's
instances through it, and a `Self` parameter takes them:

<!-- typy-expect: accepted -->
```python
from typing import Self


class Shape:
    def __init__(self, size: float) -> None:
        self.size = size

    def grown(self, factor: float) -> Self:
        self.size = self.size * factor
        return self

    @classmethod
    def unit(cls) -> Self:
        return cls(1.0)


class Circle(Shape):
    def label(self) -> str:
        return f"circle {self.size}"


print(Circle(1.0).grown(2.0).label(), Circle.unit().label())
```

In the body, `self`, a parameter declared `Self` and what a method that
returns `Self` gives through them are of `Self`; another instance of the
class is E0433, since a call through a derived class would get it in place
of an instance of that class. A class method's `cls` makes its own class,
so `cls(...)` is of `Self`. `Self` in a static method, in a generic or a
protocol class, in an attribute's annotation and inside another type, such
as `list[Self]`, is planned for 0.13 (TYP-36).

## Comparison methods

`__eq__`, `__ne__`, `__lt__`, `__le__`, `__gt__` and `__ge__` give a
class's instances `==`, `!=`, `<`, `<=`, `>` and `>=`, and `__hash__`
their hash (CLS-07). Lists, sets, dicts, `in`, `sorted`, `min` and
`max` call them too, as CPython does:

<!-- typy-expect: accepted -->
```python
class Version:
    def __init__(self, major: int, minor: int) -> None:
        self.major = major
        self.minor = minor

    def __eq__(self, other: "Version") -> bool:
        return (self.major, self.minor) == (other.major, other.minor)

    def __lt__(self, other: "Version") -> bool:
        return (self.major, self.minor) < (other.major, other.minor)

    def __hash__(self) -> int:
        return self.major * 1000 + self.minor


versions = [Version(2, 0), Version(1, 5)]
print(Version(1, 5) in versions, sorted(versions)[0].major)
print(len({Version(1, 5), Version(1, 5)}), Version(2, 0) > Version(1, 5))
```

A comparison method takes `self` and one value its class's instances
are, and returns a `bool`; `__hash__` returns an `int`. As in CPython,
`a > b` falls back to `b.__lt__(a)`, `a != b` to `not a == b`, the
right operand's method comes first when its class derives from the
left's, and a class that defines `__eq__` without `__hash__` has no
hash, so a set or a dict key of it is E0413. A container compares an
instance with an object its `__eq__` does not take as unequal.

A comparison or hash method may raise, also when a list, a set, a dict,
`sorted`, `min` or `max` calls it: the exception leaves the operation at
once, no later method runs, and a set or a dict keeps the items it had,
as in CPython. A chain such as `low < value < high` calls each
comparison's method and evaluates `value` once (CLS-22). `sort` and
`sorted` call `__lt__` with the same items in the same order as CPython
3.15 does; CPython 3.12, 3.13 and 3.14 each call it in another order
(OI-27).

## Arithmetic methods

`__add__`, `__sub__`, `__mul__`, `__truediv__`, `__floordiv__`,
`__mod__`, `__pow__`, `__matmul__`, `__lshift__`, `__rshift__`,
`__and__`, `__or__` and `__xor__` give the binary operators, each with
its reflected form such as `__radd__`; `__neg__`, `__pos__` and
`__invert__` give `-x`, `+x` and `~x`, and `__abs__` gives `abs(x)`
(CLS-07):

<!-- typy-expect: accepted -->
```python
class Money:
    def __init__(self, cents: int) -> None:
        self.cents = cents

    def __add__(self, other: "Money") -> "Money":
        return Money(self.cents + other.cents)

    def __rmul__(self, times: int) -> "Money":
        return Money(self.cents * times)


total = Money(250)
total += Money(100)
print(total.cents, (3 * total).cents)
```

As in CPython, `3 * total` falls back to the right operand's
`__rmul__`, and the right operand's reflected method comes first when
its class derives from the left's and defines it again. When the
reflected method may run, the operator's type joins the two methods'
results, such as `int | str` (CLS-22). `x += y` calls
the class's in-place method, such as `__iadd__`, when it defines one,
and `x = x + y` otherwise, for any target: an item or an attribute of an
expression is evaluated once, as in CPython (CLS-20). CPython calls the
in-place method whatever `y` is, so an in-place method whose parameter
does not take `y` is refused (E0404), never passed over for `__add__`.
An in-place method that only a subclass of the target's class defines is
planned for 0.14 (CLS-32).

## Giving way with `NotImplemented`

An operator or comparison method returns `NotImplemented` when it does
not handle the other operand, and the operator then tries the other
operand's method, as CPython does (CLS-21):

<!-- typy-expect: accepted -->
```python
class Meters:
    def __init__(self, value: float) -> None:
        self.value = value

    def __add__(self, other: "Meters | Feet") -> "Meters":
        if isinstance(other, Meters):
            return Meters(self.value + other.value)
        return NotImplemented

    def __eq__(self, other: "Meters | Feet") -> bool:
        if isinstance(other, Meters):
            return self.value == other.value
        return NotImplemented


class Feet:
    def __init__(self, value: float) -> None:
        self.value = value

    def __radd__(self, other: "Meters | Feet") -> Meters:
        if isinstance(other, Meters):
            return Meters(other.value + self.value * 0.3048)
        return NotImplemented


print((Meters(1.0) + Feet(10.0)).value, Meters(1.0) == Feet(1.0))
print(Meters(2.0) in [Meters(1.0), Meters(2.0)])
try:
    print(Feet(1.0) + Feet(2.0))
except TypeError as error:
    print(error)
```

When every method gives way, `==` and `!=` compare identities, and any
other operator raises CPython's `TypeError`. `x += y` tries the in-place
method first, then the methods of `x + y`. A method that returns
`NotImplemented` gives every method of its name in the classes derived
from the same base the return type `T | NotImplementedType`, so a direct
call such as `a.__eq__(b)` may give `NotImplemented`, which
`result is NotImplemented` tests. A method that may return both `None`
and `NotImplemented` is planned for 0.14 (CLS-33), and `return
NotImplemented` in any other function is E0405.

## Container methods

`__len__` gives `len(x)`, `__getitem__`, `__setitem__` and `__delitem__`
give `x[k]`, `x[k] = v` and `del x[k]`, `__contains__` gives `v in x`,
and `__bool__` gives the truth of `if x`, `not x`, `bool(x)`, `any` and
`all`; without `__bool__`, an instance is true when `__len__` is not
zero (CLS-07):

<!-- typy-expect: accepted -->
```python
class Shelf:
    def __init__(self) -> None:
        self.books: dict[str, int] = {}

    def __len__(self) -> int:
        return len(self.books)

    def __getitem__(self, title: str) -> int:
        return self.books[title]

    def __setitem__(self, title: str, pages: int) -> None:
        self.books[title] = pages

    def __contains__(self, title: str) -> bool:
        return title in self.books


shelf = Shelf()
shelf["Dune"] = 412
print(len(shelf), shelf["Dune"], "Dune" in shelf, bool(Shelf()))
```

As in CPython, a negative `__len__` raises `ValueError`, and `in` on an
instance whose class has `__iter__` and no `__contains__` goes over its
items until one is the value or equal to it (CLS-23). A slice such as
`x[1:2]` passes a `slice` object to `__getitem__`, `__setitem__` or
`__delitem__`, which a parameter annotated `slice` or `int | slice`
takes, and `x[k] += v` calls `__getitem__`, then `__setitem__`, with
`x` and `k` evaluated once:

<!-- typy-expect: accepted -->
```python
class Row:
    def __init__(self, cells: list[int]) -> None:
        self.cells = cells

    def __getitem__(self, part: slice) -> "Row":
        return Row(self.cells[part])


class Tally:
    def __init__(self) -> None:
        self.counts = [0, 0]

    def __getitem__(self, index: int) -> int:
        return self.counts[index]

    def __setitem__(self, index: int, value: int) -> None:
        self.counts[index] = value


print(Row([1, 2, 3, 4])[1:3].cells)
tally = Tally()
tally[1] += 5
print(tally.counts)
```

An instance's truth comes from the `__bool__` of its own class, or its
`__len__` when it has no `__bool__`, also where the variable's type is
a base that has neither, or a union, and in `any` and `all` (CLS-24).
`in` and `for` over a class with `__getitem__` alone, which CPython
reads by position until `IndexError`, are planned for 1.0 (CST-11).

## Dataclasses

`@dataclass` from `dataclasses` writes the methods CPython writes for a
class: `__init__` taking the fields in order, `__repr__`, and `__eq__`;
with `order=True` also `__lt__`, `__le__`, `__gt__` and `__ge__`, which
compare the fields as a tuple; with `frozen=True` the fields cannot be
assigned after `__init__`; and unless `match_args=False`,
`__match_args__`, the names of the fields, which class patterns in
`match` read (CLS-05, CST-06, LIB-06):

<!-- typy-expect: accepted -->
```python
from dataclasses import dataclass, field


@dataclass(order=True)
class Version:
    major: int
    minor: int = field(default=0)


@dataclass(frozen=True)
class Token:
    kind: str
    text: str


print(sorted([Version(2), Version(1, 5)]), Version(1) == Version(1, 0))
print(Token("name", "x"))
```

The fields are the annotated names of the class body, after those of a
dataclass base, `ClassVar` ones excepted. As in CPython, a field
without a default after one with a default, a mutable default such as
`[]`, `order=True` without `eq`, a frozen dataclass deriving from a
non-frozen one or the reverse, and assigning a field of a frozen
instance are errors. A method the class defines itself is kept.

`field(default_factory=f)` calls `f`, a class or a function that takes
no arguments, for each instance whose `__init__` is not given the field
(CLS-15). A frozen dataclass, or one given `unsafe_hash=True`, hashes as
the tuple of its fields, so it can be an item of a set or a key of a
dict, and one with `eq=True` that is neither cannot (CLS-16). The
options `init=False`, `repr=False`, `unsafe_hash` and `slots` are taken
too (CLS-17):

<!-- typy-expect: accepted -->
```python
from dataclasses import dataclass, field


@dataclass(frozen=True)
class Point:
    x: int
    y: int = 0


@dataclass
class Path:
    name: str
    points: list[Point] = field(default_factory=list)


first = Path("a")
first.points.append(Point(1))
print(first, Path("b"), {Point(1, 2), Point(1, 2)})
```

The other options of `field`, such as `repr` or `compare`, `kw_only` and
`weakref_slot` are planned for 0.14 (CLS-28).

## Named tuples

A class derived from `NamedTuple` of `typing` declares its fields as
annotated names, each with a constant default or none. Its instances are
tuples of the fields: they are built by position or by name, read by
field name, and printed, compared and hashed as the tuple of their fields
is, also with a plain tuple or another named tuple, as in CPython
(CLS-27, ADR-0088):

<!-- typy-expect: accepted -->
```python
from typing import NamedTuple


class Point(NamedTuple):
    x: int
    y: int = 0

    def norm(self) -> int:
        return abs(self.x) + abs(self.y)


p = Point(3, -4)
x, y = p
print(p, p.x, p[1], len(p), p.norm(), x + y)
print(p == (3, -4), sorted([p, Point(1)]), isinstance(p, tuple))
match p:
    case Point(x=3, y=b):
        print("x is 3 and y is", b)
```

An index, a slice, `len`, `in`, a loop, unpacking, a sequence pattern,
`+` and a comparison read the named tuple as the tuple of its fields,
with the rules of a fixed tuple: an index is an integer literal, and a
loop needs fields of one type. The class body may hold methods,
properties, class methods, static methods and class variables, and a
class derived from a named tuple may add methods. A generic named tuple,
such as `class Pair[T](NamedTuple)`, is copied for its type arguments
as any generic class is. `sorted` of a list of named tuples needs fields
that order, such as numbers, strings, or lists or tuples of them.

As in CPython, a field without a default after one with a default, a
field name that starts with an underscore, a `ClassVar` field, a member
named `__init__`, `__new__`, `_replace` or another name `NamedTuple`
keeps, and a second base are errors, reported with CPython's text.
Assigning a field is refused, where CPython raises `AttributeError:
can't set attribute`. A named tuple where a plain tuple is expected is
refused; `tuple(p)` gives the plain tuple:

<!-- typy-expect: rejected E0414 at 9:1 -->
```python
from typing import NamedTuple


class Point(NamedTuple):
    x: int


point = Point(1)
point.x = 5
```

`_fields` holds the names of the fields, and `_asdict()` gives a dict of
them with their values. `_replace` makes a new instance with the fields
it is given by name and the others of the instance it is called on, of
that instance's own class, also where the variable's type is a base;
a name that is no field is refused with CPython's text:

<!-- typy-expect: accepted -->
```python
from typing import NamedTuple


class Point(NamedTuple):
    x: int
    y: int = 0


class Labelled(Point):
    def label(self) -> str:
        return f"P{self.x}"


def moved(point: Point) -> Point:
    return point._replace(x=point.x + 1)


print(moved(Point(1, 2)), moved(Labelled(5)), Point._fields)
print(Point(3)._asdict())
```

`_make`, `_field_defaults`, and `_replace` read as a value, called
through the class or given `**` arguments are planned for 0.14 (CLS-43).
A named tuple made by a call, such as `NamedTuple("P", [("x", int)])`, a
class derived from a named tuple with `__new__`, `__init__` or
attributes of its own, and one that defines `__len__`, `__getitem__`,
`__iter__` or `__contains__` are planned for 1.0 (CLS-41), and a named
tuple where a plain tuple is expected also for 1.0 (CLS-42).

## Abstract classes

A class derived from `abc.ABC` marks a method `@abstractmethod`, and a
class derived from it overrides the method (CLS-45). An instance is made
only of a class that overrides every abstract method it has, so a call
through an instance always runs an override:

<!-- typy-expect: accepted -->
```python
from abc import ABC, abstractmethod


class Shape(ABC):
    @abstractmethod
    def area(self) -> float: ...

    @property
    @abstractmethod
    def sides(self) -> int: ...

    def describe(self) -> str:
        return f"{self.area():.1f} with {self.sides} sides"

    @classmethod
    def make(cls) -> "Shape":
        return cls()


class Square(Shape):
    def area(self) -> float:
        return 4.0

    @property
    def sides(self) -> int:
        return 4


print(Square().describe(), Square.make().describe())
try:
    Shape.make()
except TypeError as error:
    print(error)
```

A call that names a class with an abstract method left, such as
`Shape()`, is E0404. Through a value that holds the class, such as
`cls()` in a class method called through `Shape`, it raises CPython's
`TypeError` when the program runs. `@abstractmethod` comes last, just
above `def`: above `@property`, `@staticmethod` or `@classmethod`
CPython raises `AttributeError` when the class is defined, and TyPy
refuses it with E0434.

An abstract method's body may hold only a docstring, `pass` or `...`.
CPython returns `None` when such a body runs, so where the method's
result does not hold `None`, a call that runs the body directly is
E0434: `super().area()`, `Shape.area(square)`, and `Shape.area` read as
a value. A call through an instance runs the override. An abstract
static or class method, which a call through the class runs, gives a
value or raises, as other functions do (E0405). Without `ABC` among the
class's ancestors, `@abstractmethod` changes nothing, as in CPython.
`ABC` and `abstractmethod` are the names of `abc` TyPy gives;
`ABCMeta` as a metaclass, and `ABC` as a type or a value, are refused.

## Enum classes

A class derived from `enum.Enum` names its members in its body, each a
constant or `auto()` (CLS-44). A member is an instance of the class, with
`name` and `value`; a member whose value another member has already is
an alias of that member. The class finds a member by its value with a
call and by its name with a subscript, and a loop, `len` and `in` read
its members, aliases left out:

<!-- typy-expect: accepted -->
```python
from enum import Enum, auto


class Color(Enum):
    RED = 1
    GREEN = auto()
    BLUE = auto()
    CRIMSON = 1

    def label(self) -> str:
        return self.name.lower()


def warmth(color: Color) -> str:
    match color:
        case Color.RED:
            return "warm"
        case _:
            return "cool"


print(Color.RED, repr(Color.GREEN), Color.BLUE.value)
print(Color(2), Color["CRIMSON"] is Color.RED, len(Color))
print([color.label() for color in Color], warmth(Color.BLUE))
try:
    Color(7)
except ValueError as error:
    print(error)
```

Members compare and hash as themselves, so they serve as dict keys and
`match` cases. A member's value is an `int`, `str`, `bytes` or `bool`
constant, or `auto()`, which gives one more than the largest value
before it, or `1` for the first member; `value` has the union of the
members' value types. `Color(value)` takes a value of that type, and a
value no member has raises CPython's `ValueError`; `Color[name]` raises
`KeyError` for a name no member has.

A member of an `IntEnum` or a `StrEnum` is also its value, an `int` or a
`str`: arithmetic, comparisons, an index, the builtins of numbers and
text, a parameter or a variable declared `int` or `str`, the methods of
`str`, and an f-string field with a format specification read its value,
while `repr` still shows the member (ADR-0117). `auto()` of a `StrEnum`
gives the member's name in lower case:

<!-- typy-expect: accepted -->
```python
from enum import IntEnum, StrEnum, auto


class Level(IntEnum):
    LOW = auto()
    HIGH = 10


class Mode(StrEnum):
    READ = auto()
    WRITE = "w"


def double(n: int) -> int:
    return n * 2


print(repr(Level.HIGH), Level.HIGH + 1, double(Level.LOW), [7, 8][Level.LOW])
print(sorted([Level.HIGH, Level.LOW]), f"{Level.HIGH:04}", Level.LOW == 1)
print(repr(Mode.READ), Mode.READ, Mode.READ.upper(), Mode.WRITE == "w")
```

A list of members is not a list of their values, so
`", ".join(list(Mode))` is E0404; `[m.value for m in Mode]` gives the
values.

The members of a `Flag` or an `IntFlag` are bits, each one bit or
`auto()`, which gives the bit above the highest one before it. `|`, `&`,
`^` and `~` give a member, or a composite named by the members whose bits
it holds; each composite is made once, so `is` holds between two of the
same value, as in CPython (ADR-0118). A loop over a flag, `in` and `len`
read its bits. A `Flag` of a value with bits no member has raises
CPython's `ValueError`, and an `IntFlag` keeps them, and is also its
`int` value as an `IntEnum` is:

<!-- typy-expect: accepted -->
```python
from enum import Flag, IntFlag, auto


class Perm(Flag):
    READ = auto()
    WRITE = auto()
    RUN = auto()


class Bits(IntFlag):
    LOW = 1
    HIGH = 2


both = Perm.READ | Perm.WRITE
print(repr(both), both, both.name, list(both), Perm.RUN in both)
print(repr(~Perm.READ), both is Perm(3), repr(Perm(0)), len(both))
print(repr(Bits.LOW | 8), Bits.HIGH + 1, repr(~Bits.LOW))
try:
    Perm(8)
except ValueError as error:
    print(error)
```

What CPython refuses when the class statement runs is E0404, such as a
member defined twice, a class derived from an enum class with members,
or a `StrEnum` member whose value is not a `str`. `auto()` after a value
that is not an `int` is E0202, as CPython 3.12 gives `1` with a warning
and 3.13 raises `TypeError`. Members
of other values, annotated members, `__init__`, `__new__`, `_missing_`,
other bases, decorators such as `@unique`, flag members of other than
one bit, and classes derived from an enum class without members are
planned for 0.13 (CLS-47).

## Attributes are declared

An attribute the class does not declare cannot be added:

<!-- typy-expect: rejected E0414 at 5:7 -->
```python
class Point:
    def __init__(self) -> None:
        self.x = 0
point = Point()
point.z = 1
```
