# typy-test: EXP-22
# typy-expect: accepted
def pop(values: list[int], where: list[int]) -> str:
    try:
        return str(values.pop(*where))
    except (TypeError, IndexError) as error:
        return str(error)


def find(text: str, places: list[int]) -> str:
    try:
        return str(text.find("b", *places))
    except TypeError as error:
        return str(error)


print(pop([1, 2, 3], []), pop([1, 2, 3], [0]), pop([1], [0, 1]))
print(find("abcb", []), find("abcb", [2]), find("abcb", [0, 2]))
words = ["x", "y"]
words.append(*["z"])
print(words, "-".join(*[words]))
seen = {1, 2}
others = [[2, 3], [4]]
print(sorted(seen.union(*others)), sorted(seen.intersection(*[[1, 2, 5], [2]])), sorted(seen.difference(*others)))
seen.update(*others)
print(sorted(seen))
empty: list[list[int]] = []
print(sorted(seen.union(*empty)), sorted(seen.intersection(*empty)))
seen.difference_update(*[[1], [2]])
seen.intersection_update(*[[3, 4, 9]])
print(sorted(seen))
try:
    words.append(*["a", "b"])
except TypeError as error:
    print(error)
nothing: list[str] = []
print("  pad ".strip(*nothing), "abc".upper(*nothing))
