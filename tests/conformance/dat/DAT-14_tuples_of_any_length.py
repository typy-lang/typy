# typy-test: DAT-14
# typy-expect: accepted
def total(values: tuple[int, ...]) -> int:
    result = 0
    for value in values:
        result += value
    return result


def pair() -> tuple[str, ...]:
    return ("a", "b")


t: tuple[int, ...] = (3, 1, 2)
print(t, len(t), t[0], t[-1], t[1:], t[::-1], total(t), total((5, 6)), total(()))
print(pair(), tuple([1, 2]), tuple("ab"), tuple(), tuple(reversed(t)), tuple(enumerate("xy")))
u = tuple([7])
print(u, u + t, t * 2, 3 in t, 9 not in t, t.count(1), t.index(2), sorted(t), max(t), sum(t))
print(t == (3, 1, 2), t < (3, 2), (3, 1, 2) == t, t != tuple([3, 1, 2]), list(t))
nested: list[tuple[int, ...]] = [(1, 2), (), (3,)]
print(nested, sorted(nested), nested[2], bool(t), bool(tuple()))
empty: tuple[str, ...] = tuple()
print(empty, repr(empty), str(u))
print(t[5])
