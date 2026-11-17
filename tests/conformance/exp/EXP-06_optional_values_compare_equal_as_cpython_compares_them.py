# typy-test: EXP-06
# typy-expect: accepted
from dataclasses import dataclass


def same(left: int | None, right: int | None) -> bool:
    return left == right


def differ(left: str | None, right: str) -> bool:
    return left != right


def pair(value: int | None) -> tuple[int, int | None]:
    return (1, value)


@dataclass
class Reading:
    value: float | None
    label: str


print(same(1, 1), same(1, 2), same(None, None), same(None, 1), same(1, None))
print(differ(None, "a"), differ("a", "a"), differ("a", "b"))
print((1, None) == (1, None), pair(None) == pair(None), pair(2) == pair(None))
print(pair(3) != pair(3), pair(None) != pair(4))
print(Reading(None, "a") == Reading(None, "a"), Reading(1.5, "a") == Reading(None, "a"))
print(Reading(2.0, "b") != Reading(2.0, "b"))
