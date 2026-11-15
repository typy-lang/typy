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
| `__eq__`, `__ne__`, `__lt__`, `__le__`, `__gt__`, `__ge__` and `__hash__` | CLS-07 |
| `__add__`, `__radd__` and the other arithmetic, bitwise and unary methods | CLS-07 |
| `__len__`, `__getitem__`, `__setitem__`, `__delitem__`, `__contains__` and `__bool__` | CLS-07 |
| `@dataclass`, with `eq`, `order`, `frozen`, `match_args` and `field(default=...)` | CLS-05, CST-06 |
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
[Exceptions](exceptions.md) describes (CLS-03).

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
and the result is one the base's result type accepts, as type checkers
require; a mismatch is E0404. A parameter keeps the base's default, and
an override that takes or returns a type held differently from the
base's, such as `int | None` where the base has `int`, is planned for
0.5; both are E0201. `__init__` is not compared, since it runs only on
the class being made.

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
method is E0414, and `super(Base, self)` naming another class is
planned for 0.5.

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
counts as `int`, as in CPython. A class held in a variable, and
`issubclass`, are planned for 0.5.

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
`@staticmethod`. Redefining a static method in a subclass, and
assigning a property other than as `variable.name = value`, are planned
for 0.5.

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

A class method is called through its class. Calling it through an
instance, and through a subclass defined in another module than the
class that defines the method, are planned for 0.5, and so is
assigning a class variable through a subclass that inherits it, which
in CPython gives the subclass a variable of its own.

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
left's and defines it again, and a class that defines `__eq__` without
`__hash__` has no hash, so a set or a dict key of it is E0413. A
container compares an instance with an object its `__eq__` does not
take as unequal. A comparison or hash method that may raise, a chain
such as `a < b < c` of such instances, and a method returning
`NotImplemented` are planned for 0.5.

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
its class derives from the left's and defines it again. `x += y` is
`x = x + y` for a variable or an attribute of one; the in-place methods
such as `__iadd__` are planned for 0.5.

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

As in CPython, a negative `__len__` raises `ValueError`. A slice such
as `x[1:2]`, `any` and `all` of instances whose truth comes from
`__len__`, and a truth test whose static class has neither method while
a subclass defines one, are planned for 0.5, and so is `in` without
`__contains__`, which CPython answers by iterating.

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
instance are errors. A method the class defines itself is kept. The
hash of a frozen dataclass, `field(default_factory=...)` and the other
options such as `slots=True` are planned for 0.5.

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
