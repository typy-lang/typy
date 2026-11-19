# typy-test: SMT-09, FUN-04
# typy-expect: accepted
from typing import Callable


def counter() -> tuple[Callable[[], int], Callable[[], int]]:
    count = 0

    def increment() -> int:
        nonlocal count
        count += 1
        return count

    def read() -> int:
        return count
    return (increment, read)


def outer_sees_each_change() -> list[str]:
    seen: list[str] = []
    label = "start"

    def rename(new: str) -> None:
        nonlocal label
        label = new

    for word in ["a", "b"]:
        rename(word * 2)
        seen.append(label)
    return seen


def through_the_function_between() -> int:
    total = 1

    def middle() -> int:
        def inner() -> None:
            nonlocal total
            total = total * 10
        inner()
        inner()
        return total
    return middle() + total


def set_to_none() -> str:
    value: int | None = 3

    def clear() -> None:
        nonlocal value
        value = None
    value = 4
    clear()
    current = value
    return "none" if current is None else str(current + 1)


def read_before_assigned() -> str:
    def read() -> int:
        nonlocal late
        return late
    try:
        read()
    except NameError as error:
        return str(error)
    late = 1
    return str(late)


def loop_variable() -> list[int]:
    out: list[int] = []
    for i in range(3):
        def scale() -> None:
            nonlocal i
            i = i * 100
        scale()
        out.append(i)
    return out


def replaced_while_read() -> str:
    words = "x" * 50

    def replace() -> int:
        nonlocal words
        words = "y"
        return 1
    return words[:2] + str(replace()) + words


def declared_then_assigned() -> int:
    found: int

    def search() -> None:
        nonlocal found
        for found in range(5):
            if (found := found * 2) > 4:
                return
    found = -1
    search()
    return found


increment, read = counter()
increment()
increment()
print(read(), increment(), read())
print(outer_sees_each_change(), through_the_function_between())
print(set_to_none(), read_before_assigned())
print(loop_variable(), replaced_while_read(), declared_then_assigned())
