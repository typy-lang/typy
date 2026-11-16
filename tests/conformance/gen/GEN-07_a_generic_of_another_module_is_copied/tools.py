from dataclasses import dataclass
from typing import Protocol

import util


class Named(Protocol):
    def name(self) -> str: ...


@dataclass
class Pair[K, V]:
    key: K
    value: V


class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item

    def get(self) -> T:
        return self.item


class Maker:
    def make[T](self, item: T) -> list[T]:
        return [item, item]


def _twice[T](item: T) -> list[T]:
    return [item, item]


def doubled[T](item: T) -> list[T]:
    return _twice(item)


def greet(thing: Named) -> str:
    return "hello " + thing.name()


def loud[T: Named](thing: T) -> T:
    print(thing.name().upper())
    return thing


def grown[T: (int, float)](value: T) -> T:
    return util.scaled(value) + 1
