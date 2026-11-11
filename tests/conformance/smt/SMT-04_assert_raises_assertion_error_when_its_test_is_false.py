# typy-test: SMT-04, STD-05
# typy-expect: accepted
def first(items: list[int] | None) -> int:
    assert items is not None, "items are required"
    assert len(items) > 0
    return items[0]


def describe(n: int) -> str:
    calls: list[int] = []
    assert n >= 0, f"negative: {n} {calls.append(n)}"
    return str(calls)


print(first([4, 5]), describe(3))
try:
    empty: list[int] = []
    first(empty)
except AssertionError as error:
    print("empty:", repr(error))
try:
    first(None)
except AssertionError as error:
    print("none:", error)
assert first([1]) == 2, ("mismatch", 2)
