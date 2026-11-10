# typy-test: EXP-12, CST-03
# typy-expect: accepted
def squares(limit: int) -> list[int]:
    return [n * n for n in range(limit) if n % 2 == 0]


print(squares(10), [c.upper() for c in "abc"])
x = "outer"
print([x for x in range(3)], x)
words = ["apple", "fig", "kiwi", "banana"]
print({len(w) for w in words}, {w: len(w) for w in words if len(w) > 3})
print([(i, j) for i in range(3) for j in range(i) if (i + j) % 2 == 1])
print([[y * x2 for y in range(x2)] for x2 in range(4)])
grid = [[1, 2], [3, 4]]
print([cell for row in grid for cell in row], sum([v for v in range(101)]))
pairs = {(k, v) for k, v in {"a": 1, "b": 2}.items()}
print(sorted(pairs), {v: k for k, v in {"a": 1, "b": 2}.items()})
print([i for i, w in enumerate(words) if "i" in w], [a + b for a, b in zip([1, 2], [10, 20])])
print({n: [m for m in range(n)] for n in range(3)}, [w[0] for w in reversed(words)])
nested: list[list[int]] = [[] for _ in range(2)]
nested[0].append(5)
print(nested, [len(w) for w in sorted(words)], max([len(w) for w in words]))
total = 0
values = [total + n for n in range(4)]
print(values, {c for c in "hello"} == set("hello"))
counts = {w: words.count(w) for w in words}
print(counts, list({n % 3 for n in range(10)}))
print([v for v in [3, 1, 2] if v > 1 if v < 3], [b for b in b"ab"])
print([1 / n for n in [1, 0]])
