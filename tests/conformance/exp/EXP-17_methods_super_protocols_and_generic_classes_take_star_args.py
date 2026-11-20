# typy-test: EXP-17
# typy-expect: accepted
from typing import Protocol


class Logger:
    def log(self, level: int, *parts: str, **extra: str) -> str:
        return f"{level} {' '.join(parts)} {extra}"


class Loud(Logger):
    def log(self, level: int, *parts: str, **extra: str) -> str:
        return "LOUD " + super().log(level, "x", tag="t")


class Writer(Protocol):
    def log(self, level: int, *parts: str, **extra: str) -> str: ...


def use(writer: Writer) -> str:
    return writer.log(2, "via", "protocol", key="v")


class Pair[T]:
    def __init__(self, *items: T) -> None:
        self.items = items

    def with_more(self, *more: T) -> tuple[T, ...]:
        return self.items + more


logger: Logger = Loud()
print(logger.log(1, "a", "b", user="me"))
print(Logger().log(0))
print(use(Logger()), use(Loud()))
active = Logger()
print(active.log(3))
pair = Pair(1, 2)
print(pair.with_more(3, 4), Pair[str]().with_more())
