# typy-test: EXP-22
# typy-expect: accepted
def bounds(values: list[int]) -> str:
    try:
        return str(list(range(*values)))
    except TypeError as error:
        return str(error)


def size(texts: list[str]) -> str:
    try:
        return str(len(*texts))
    except TypeError as error:
        return str(error)


def absolute(values: list[float]) -> str:
    try:
        return str(abs(*values))
    except TypeError as error:
        return str(error)


print(bounds([]), bounds([3]), bounds([1, 4]), bounds([0, 9, 3]))
print(bounds([1, 2, 3, 4]))
print(size(["abc"]), size([]), size(["a", "b"]))
print(absolute([-2.5]), absolute([]))
words = ["b", "a"]
print(sorted(*[words]), sorted(*[words], reverse=True))
nothing: list[int] = []
print(bool(*nothing), bool(*[3]), float(*["2.5"]), repr(*["x"]))
print(chr(*[65]), ord(*["A"]))
