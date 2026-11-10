# typy-test: TYP-06, DAT-09
# typy-expect: accepted
ages = {"ann": 31, "bob": 42}
found = ages.get("ann")
missing = ages.get("cid")
print(found, missing)
if found is not None:
    print(found + 1)
pairs: dict[str, tuple[int, str]] = {"a": (1, "x")}
print(pairs.get("a"), pairs.get("b"))
notes: dict[str, str | None] = {"a": None}
print(notes.setdefault("b"), notes, notes.get("a"), notes.get("zz"))
nested = notes.get("b")
print(nested is None)
flags: dict[str, bool | None] = {"a": None, "b": False}
print(flags.get("a"), flags.get("b"), flags.get("c"), flags.setdefault("d"), flags)
spans: dict[int, tuple[int, int] | None] = {1: None, 2: (3, 4)}
print(spans.get(1), spans.get(2), spans.get(3))
