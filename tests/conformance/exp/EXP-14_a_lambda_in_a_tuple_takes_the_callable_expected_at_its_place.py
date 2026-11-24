# typy-test: EXP-14
# typy-expect: accepted
# A `lambda` in a tuple display takes the `Callable` the tuple's type expects at its place:
# in a list of pairs, a variable, an argument, a result and a `tuple[T, ...]`.
from collections.abc import Callable

type Step = tuple[str, Callable[[int], int]]


def run(steps: list[Step], start: int) -> int:
    value = start
    for name, step in steps:
        value = step(value)
        print(name, value)
    return value


def halving() -> Step:
    return ("half", lambda x: x // 2)


steps: list[Step] = [("double", lambda x: x * 2), ("square", lambda x: x * x)]
steps.append(halving())
pair: tuple[str, Callable[[int], int]] = ("negate", lambda x: -x)
chain: tuple[Callable[[int], int], ...] = (lambda x: x + 1, lambda x: x * 3)
table: dict[str, Step] = {"inc": ("inc", lambda x: x + 1)}
print(run(steps, 3), pair[1](4), run([("add", lambda x: x + 1)], 9))
print(chain[1](chain[0](2)), table["inc"][1](41), len(chain))
