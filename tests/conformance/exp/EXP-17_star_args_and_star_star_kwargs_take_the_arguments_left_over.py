# typy-test: EXP-17
# typy-expect: accepted
def total(*numbers: int) -> int:
    result = 0
    for number in numbers:
        result += number
    return result


def describe(name: str, *tags: str, sep: str = ", ", **fields: int) -> str:
    parts = [name, sep.join(tags)]
    for key in fields:
        parts.append(f"{key}={fields[key]}")
    return " | ".join(parts)


def noted(label: str) -> int:
    print("evaluating", label)
    return len(label)


def order(a: int, /, **rest: int) -> str:
    return f"{a} {rest}"


def keywords(**options: str) -> dict[str, str]:
    return options


class Shape:
    def __init__(self, name: str, *sides: float) -> None:
        self.name = name
        self.sides = sides

    def scaled(self, *factors: float, **named: float) -> list[float]:
        out: list[float] = []
        for side in self.sides:
            value = side
            for factor in factors:
                value *= factor
            out.append(value)
        for key in named:
            out.append(named[key])
        return out


print(total(), total(1), total(1, 2, 3))
print(describe("box"), describe("box", "a", "b", sep="/", width=2, height=3))
print(describe("x", str(noted("one")), z=noted("three"), sep=str(noted("two")), y=noted("four")))
print(order(1, a=2, b=3))
print(keywords(), keywords(first="1", second="2"))
shape = Shape("tri", 3.0, 4.0, 5.0)
print(shape.name, shape.sides, type(shape.sides).__name__)
print(shape.scaled(), shape.scaled(2.0), shape.scaled(2.0, 0.5, extra=1.5))
print(type(keywords()).__name__)


from typing import Iterator


def each(*items: int, **more: int) -> Iterator[int]:
    for item in items:
        yield item
    for key in more:
        yield more[key]


print(list(each(1, 2, x=3)))


def nested_user(prefix: str) -> str:
    def join(*words: str) -> str:
        return prefix + "-".join(words)
    return join("a", "b") + join()


print(nested_user(">"))


def first[T](*items: T) -> T:
    return items[0]


print(first(4, 5), first("a"))
