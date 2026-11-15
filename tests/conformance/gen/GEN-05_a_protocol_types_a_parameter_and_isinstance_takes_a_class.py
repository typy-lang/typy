# typy-test: GEN-05
# typy-expect: accepted
from typing import Protocol


class Named(Protocol):
    def name(self) -> str: ...


class Dog:
    def name(self) -> str:
        return "dog"


def greet(item: Named) -> str:
    return "hello " + item.name()


dog = Dog()
print(greet(dog), isinstance(dog, Dog))
