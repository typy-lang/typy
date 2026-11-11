# typy-test: CST-09, STD-05
# typy-expect: accepted
items = [1, 2, 3]
it = iter(items)
print(next(it), next(it))
items.append(4)
print(list(it), next(it, -1))
chars = iter("h\u00e9llo")
print(next(chars), next(chars), [c for c in chars], type(chars).__name__, type(iter("ab")).__name__)
scores = {"a": 1, "b": 2}
keys = iter(scores)
values = iter(scores.values())
pairs = iter(scores.items())
print(next(keys), next(values), next(pairs), list(pairs))
print(next(iter(b"\x05")), next(iter({7})), next(iter((1, 2))), type(iter(scores.items())).__name__)
empty: list[str] = []
print(next(iter(empty), None), next(iter(empty), "nothing"))
for n in iter([10, 20]):
    print(n)
again = iter([1])
print(iter(again) is again)
growing = {1: 1}
try:
    for key in iter(growing):
        growing[key + 1] = 0
except RuntimeError as error:
    print("error:", error)
try:
    next(iter(empty))
except StopIteration as error:
    print("stop:", repr(error))
next(iter(""))
