# typy-test: EXP-22
# typy-expect: accepted
def largest(values: list[int]) -> str:
    try:
        return str(max(*values))
    except TypeError as error:
        return str(error)


def smallest(words: list[str]) -> str:
    try:
        return min(*words, key=len)
    except TypeError as error:
        return str(error)


def capped(values: list[float]) -> str:
    try:
        return str(max(*values, default=0.0))
    except TypeError as error:
        return str(error)


print(largest([3, 9, 4]), largest([5]), largest([]))
print(smallest(["ccc", "a", "bb"]), smallest(["xyz"]), smallest([]))
print(max(2, *[7, 1]), min(*"hello"), max(*["hello"]))
print(capped([1.5, 2.5]), capped([2.5]), capped([]))
