# typy-test: DAT-09, LIB-03
# typy-expect: accepted
def scores() -> dict[str, float]:
    return {"a": 1.5, "b": -0.25}


print(scores()["a"], {1: "one"}[1], scores().get("c", 0.0), scores().get("b", 9.0))
print(scores().pop("b"), scores().pop("z", 2.0), scores().setdefault("a", 7.0))
print("a" in scores().keys(), 1.5 in scores().values(), ("b", -0.25) in scores().items())
print(len(scores().items()), bool(scores().values()), scores().popitem())
views = [scores().keys(), {"z": 0.5}.keys()]
print(views, (scores().items(), 1))
held = scores()
pairs = held.items()
while held:
    print(held.popitem(), len(pairs), pairs)
