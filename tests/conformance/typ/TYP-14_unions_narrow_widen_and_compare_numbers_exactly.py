# typy-test: TYP-14, TYP-07
# typy-expect: accepted
def size(value: int | str | bytes) -> int:
    if isinstance(value, int):
        return value
    return len(value)


def widened(value: list[int] | list[str]) -> int | list[int] | list[str]:
    return value


def found(value: int | str, values: list[int | str]) -> bool:
    return value in values and 1 == value


print(size(4), size("ab"), size(b"xyz"))
print(widened([1]), widened(["a"]))
print(found(1, [1, "a"]), found("a", [1]))
huge: list[int | float] = [10**19, 1e19, float("inf"), 2**70, 0.5]
print(2**70 in huge, float("inf") in huge, 3 in huge)
first = 0
plain: list[int] = [0]
first = plain[0] = 5
print(first, plain)
