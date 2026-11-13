# typy-test: DAT-17, CST-03, CST-09
# typy-expect: accepted
def total(numbers: range) -> int:
    result: int = 0
    for number in numbers:
        result += number
    return result


def describe(numbers: range) -> str:
    return f"{numbers!r} len={len(numbers)} true={bool(numbers)}"


def main() -> None:
    r = range(10)
    print(r, len(r), r[0], r[-1], r[3])
    s = range(2, 20, 3)
    print(repr(s), str(s), list(s), s[1:4], s[::-1], s[::2])
    print(total(s), total(range(5, 0, -1)))
    print(describe(range(0)), describe(range(3, 3)), describe(range(-5, 5, 2)))
    print(4 in r, 10 in r, -1 in r, 5 in s, 8 in s)
    print(range(0) == range(4, 4), range(0, 3) == range(3), range(1) != range(2))
    print(range(0, 10, 3) == range(0, 11, 3), range(1, 2, 5) == range(1, 2, 9))
    for value in reversed(s):
        print(value, end=" ")
    print()
    it = iter(range(3))
    print(type(it).__name__, next(it), next(it), next(it), next(it, -1))
    big = range(2**70, 2**70 + 6, 2)
    print(big, len(big), big[-1], 2**70 + 2 in big, list(big))
    print(type(iter(big)).__name__)
    print(type(r).__name__, type(r) is range, r[2:2], bool(range(2**70)))
    rs: list[range] = [range(1), range(2)]
    print(rs, {range(2): "two"}[range(0, 2)])
    if range(1):
        print("non-empty")
    try:
        print(r[10])
    except IndexError as error:
        print("IndexError", error)
    try:
        range(1, 2, 0)
    except ValueError as error:
        print("ValueError", error)
    try:
        print(len(range(2**70)))
    except OverflowError as error:
        print("OverflowError", error)


main()
