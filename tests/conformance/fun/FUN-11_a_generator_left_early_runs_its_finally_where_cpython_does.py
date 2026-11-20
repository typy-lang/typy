# typy-test: FUN-11, FUN-07
# typy-expect: accepted
from collections.abc import Iterator
from types import TracebackType


class Manager:
    def __init__(self, name: str) -> None:
        self.name = name

    def __enter__(self) -> str:
        print("enter", self.name)
        return self.name

    def __exit__(
        self,
        kind: type[BaseException] | None,
        value: BaseException | None,
        traceback: TracebackType | None,
    ) -> bool:
        print("exit", self.name, kind is GeneratorExit)
        return False


def managed(name: str) -> Iterator[str]:
    with Manager(name) as held:
        yield held
        yield held + " again"


def inner(name: str) -> Iterator[int]:
    try:
        yield 1
    finally:
        print("inner closing", name)


def outer(name: str) -> Iterator[int]:
    child = inner(name)
    next(child)
    try:
        yield 2
    finally:
        print("outer closing", name)


def quiet() -> Iterator[int]:
    try:
        yield 1
    except GeneratorExit:
        print("quiet saw GeneratorExit")


def counting(name: str) -> Iterator[int]:
    try:
        yield 1
        yield 2
    finally:
        print("counting closing", name)


def main() -> None:
    for text in managed("with"):
        print("got", text)
        break
    print("after with loop")
    for number in outer("nested"):
        print("outer gave", number)
        break
    print("after nested")
    print(next(quiet()))
    print("after quiet")
    print(next(counting("temporary")))
    print("after temporary")
    print(sum(counting("sum")))
    print(list(zip(counting("zipped"), [10])))
    print("after zip")
    unused = counting("never started")
    print(unused is not None)
    unused = counting("replacement")
    print("never started was freed")
    print(max(counting("max")))


main()
for number in counting("module loop"):
    print("module loop gave", number)
    break
else:
    print("not reached")
print("after module loop")
