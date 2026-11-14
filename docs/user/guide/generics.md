# Generics

A generic function takes a type parameter list, as PEP 695 writes it:
`def first[T](items: list[T]) -> T`. TyPy compiles one copy of the
function for each set of types the program calls it with, so a generic
call costs what a call written for those types costs (GEN-01 to GEN-03,
ADR-0023).

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `def name[T, U](...)`: a generic function at module level | GEN-01 |
| `[T: Bound]`, a bound, and `[T: (A, B)]`, constraints | GEN-02 |
| One compiled copy for each set of type arguments, with no type at run time | GEN-03 |

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

## Refused and planned

`TypeVar` and `Generic` from `typing` are E0202: a type parameter list
replaces them (GEN-01). Type parameters of methods, generic classes,
generic type aliases and protocols come later in 0.4. Calling a generic
function of another module is planned for 0.5 (GEN-07), and `*Ts`,
`**P` and defaults of type parameters for 1.0 (GEN-06).
