# typy-test: DAT-07
# typy-expect: accepted
mixed: list[tuple[bool, None, float, tuple[str, int]]] = [(True, None, -0.5, ("x", 7))]
print(mixed, mixed[0][3][0], mixed[0][1])
singles = [(1,), (2,)]
print(singles, sorted(singles, reverse=True))
nested = [[(1, 2)], []]
print(nested, any(nested), all(nested))
empty: list[tuple[int, str]] = []
print(empty, max(empty, default=(0, "none")))
print(min(empty))
