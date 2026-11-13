# typy-test: DAT-16
# typy-expect: accepted
def note(text: str, value: int) -> int:
    print("evaluate", text)
    return value


print(dict.fromkeys(["a", "b", "a"]), dict.fromkeys(range(3), 0))
print(dict.fromkeys("hello", 1), dict.fromkeys({3: "x", 1: "y"}, "v"))
shared = dict.fromkeys(["x", "y"], [0])
shared["x"].append(1)
print(shared)
print(dict.fromkeys(reversed([1, 2, 3]), note("value", 5)))
print(dict.fromkeys([n * n for n in range(4)], True))
empty: list[int] = []
print(dict.fromkeys(empty), dict.fromkeys(empty, "z"))
