# Classes

Classes are standard Python classes. Every attribute is declared in the
class body or assigned in `__init__`, and has one type.

## What TyPy accepts

| Item | Rule |
| --- | --- |
| Classes with `__init__`, methods and attributes declared in the body or assigned in `__init__` | CLS-01 |
| Assigning or calling a name the class does not have is refused | CLS-02 |
| A class derived from one class of the program, with its attributes and methods | CLS-04 |
| `__str__` and `__repr__`, used by `print`, `str`, `repr`, f-strings and containers | CLS-08 |
| A type object for every type, with `object` as the root | CLS-10 |
| `type(x)`, class and built-in type names as values, `__name__` | CLS-12 |

Inside `__init__`, `self` is only used to assign and read attributes until
every attribute holds a value, so no attribute is ever read empty. A
signature that names its own class, or a class defined below it, quotes
the name, such as `"Node | None"`.

Instances are equal only to themselves and hashed by identity; operator
methods such as `__eq__` come in 0.4 (CLS-07), with `@dataclass`
(CLS-05), `@property` (CLS-06) and `isinstance` (CLS-09).
A class may derive from an exception class, as
[Exceptions](exceptions.md) describes (CLS-03); class variables come in
0.4 (CLS-11).

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

A subclass may declare an attribute of its base again only with the
same type. The base of a class is `object`, an exception class or a
class of the program defined before it; a built-in type such as `int` or
`list` is E0201, and several bases are planned for after 1.0. A subclass
defines its own `__init__`, `__str__` and `__repr__`; redefining any
other method of a base, `super()` beyond an exception class's
`super().__init__(...)`, and `isinstance` come later in 0.4.

<!-- typy-expect: rejected E0201 at 1:13 -->
```python
class Count(int):
    pass
```

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
