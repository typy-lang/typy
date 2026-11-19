# typy-test: EXP-14, FUN-04, FUN-05
# typy-expect: accepted
from typing import Callable


def apply(f: Callable[[int], int], value: int) -> int:
    return f(value)


def adder(step: int) -> Callable[[int], int]:
    return lambda value: value + step


def late_binding() -> list[int]:
    functions: list[Callable[[], int]] = []
    for i in range(3):
        functions.append(lambda: i)
    return [each() for each in functions]


def in_comprehensions() -> list[int]:
    made = [lambda: i * 10 for i in range(3)]
    again = [lambda: i for i in range(5)]
    return [each() for each in made] + [again[0]()]


def comprehension_run_twice() -> list[int]:
    made: list[Callable[[], int]] = []
    for size in [2, 4]:
        made.extend([lambda: i for i in range(size)])
    return [each() for each in made]


def keyed(words: list[str]) -> tuple[list[str], list[str], str, str]:
    return (
        sorted(words, key=lambda word: (len(word), word)),
        sorted(words, key=lambda word: [word[-1], word]),
        min(words, key=lambda word: word[-1]),
        max(words, key=lambda word: len(word)),
    )


def nested(base: int) -> int:
    make: Callable[[int], Callable[[int], int]] = lambda a: lambda b: a * b + base
    return make(3)(4)


def chosen(flag: bool) -> Callable[[int], int]:
    return (lambda x: x + 1) if flag else (lambda x: x - 1)


def held() -> list[int]:
    listed: list[Callable[[int], int]] = [lambda x: x, lambda x: -x]
    named: dict[str, Callable[[int], int]] = {"square": lambda x: x * x}
    return [each(2) for each in listed] + [named["square"](3)]


def optional() -> int:
    pick: Callable[[int], int] | None = lambda x: x + 1
    return -1 if pick is None else pick(1)


def fails() -> str:
    check: Callable[[str], int] = lambda text: int(text)
    try:
        check("x")
    except ValueError as error:
        return "ValueError: " + str(error)
    return "unreached"


double: Callable[[int], int] = lambda x: x * 2
answer = lambda: 42
factorial: Callable[[int], int] = lambda n: 1 if n == 0 else n * factorial(n - 1)
pairs = [(1, "b"), (0, "a"), (2, "c")]
pairs.sort(key=lambda pair: pair[1])
print(apply(lambda x: x * x, 7), adder(10)(5), double(21), answer(), factorial(5))
print(late_binding(), in_comprehensions(), comprehension_run_twice())
print(keyed(["pear", "fig", "banana", "kiwi"]))
print(nested(1), optional(), pairs, fails())
print(chosen(True)(1), chosen(False)(1), held())
print(repr(double).split(" at ")[0], repr(adder(1)).split(" at ")[0])
