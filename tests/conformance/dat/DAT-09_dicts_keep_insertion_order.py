# typy-test: DAT-09, LIB-02, LIB-03
# typy-expect: accepted
def count(words: list[str]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for word in words:
        counts[word] = counts.get(word, 0) + 1
    return counts


print(count(["b", "a", "b", "c", "a", "b"]))
d = {3: "c", 1: "a", 2: "b"}
print(d, len(d), bool(d), d[1], 2 in d, 9 in d, 9 not in d)
d[1] = "A"
d[10] = "j"
print(d, d.get(10, "?"), d.get(11, "?"), d.pop(3), d, d.pop(99, "none"))
print(d.popitem(), d, d.setdefault(5, "e"), d.setdefault(5, "x"), d)
print(d.keys(), d.values(), d.items(), len(d.keys()), "e" in d.values(), (5, "e") in d.items())
for key in d:
    print(key, d[key], end="; ")
print()
for key, value in d.items():
    print(key, value, end="; ")
print()
print(sorted(d), list(d.values()), sum({1: 2, 3: 4}.values()), max(d))
e = dict(d)
e.update({7: "g"})
e.update([(8, "h"), (1, "one")])
e.update(zip([20, 21], ["u", "v"]))
print(e, e == d, e != d, d == dict(d), d | {0: "z"}, dict(enumerate("ab")))
d |= {1: "uno"}
print(d, d.copy(), d.copy() == d)
counts: dict[str, int] = {"x": 1}
counts["x"] += 5
counts["y"] = counts.get("y", 0) - 1
print(counts)
nested: dict[str, list[int]] = {"a": [1]}
nested["a"].append(2)
nested.setdefault("b", []).append(3)
print(nested, [{1: 2}, {}], ({1: 2}, 3), {(1, 2): {3.5}})
keys = d.keys()
d[42] = "late"
print(keys, list(keys)[-1], d.items())
d.clear()
print(d, keys, len(keys), bool(keys), set({4: 1, 2: 2, 9: 3}), set(e.keys()) == set(e))
words = {"one": 1, "two": 2}
print(sorted(words.items()), "-".join(words), str(words), repr(words.keys()))
