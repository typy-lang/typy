# typy-test: DAT-08, EXP-06
# typy-expect: accepted
point = (3, "three", 2.5, None, True)
print(point, point[0], point[-1], len(point))
nested = (1, ("x", (2.5, None)))
print(nested, nested[1][1][0], nested[-1][0])
print((1,), (), ("it's",), repr(("a", 1)), str((1.0, -0.0)))
print((1, 2) == (1, 2), (1, 2) != (1, 3), (1, 2) < (1, 3), (2,) <= (1,))
print((1, "b") > (1, "a"), (1, 2.0) == (1.0, 2), (2 ** 70, 1) < (2 ** 70, 2))
joined = (1, "a") + (2.5,) + ()
print(joined, len(joined), bool(()), bool((0,)))


def make() -> tuple[str, float]:
    return ("made", 0.5)


print(make()[0], make()[1], make()[-1] + 1.0, make())
print((None,) < (None,), (1, None) <= (1, None), (1, None) < (2, None))
