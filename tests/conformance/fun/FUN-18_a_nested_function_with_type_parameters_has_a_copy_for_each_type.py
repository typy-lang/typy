# typy-test: FUN-18
# typy-expect: accepted
from collections.abc import Callable, Iterator


def outer(scale: int) -> list[str]:
    def pair[T](first: T, second: T) -> tuple[T, T]:
        return (first, second)

    def times[N: (int, float)](value: N) -> N:
        return value * scale

    def largest[T](items: list[T], key: Callable[[T], int]) -> T:
        best = items[0]
        for item in items:
            if key(item) > key(best):
                best = item
        return best

    def each[T](items: list[T]) -> Iterator[T]:
        for item in items:
            yield item

    def run() -> str:
        return str(pair("x", "y"))

    shown = [str(pair(1, 2)), str(pair("a", "b")), str(times(3)), str(times(1.5))]
    shown.append(largest(["aa", "b", "cccc"], len) + run())
    shown.append(str(list(each([1.5, 2.5]))) + str(list(each(["c"]))))
    return shown


def wrap[T](value: T) -> list[T]:
    def again[U](other: U) -> tuple[T, U]:
        return (value, other)

    print(again(1), again("z"))
    return [value]


print(outer(2))
print(wrap(1.5), wrap("q"))
