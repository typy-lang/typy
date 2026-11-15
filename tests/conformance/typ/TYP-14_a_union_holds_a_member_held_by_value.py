# typy-test: TYP-14, TYP-07
# typy-expect: accepted
def describe(value: int | str) -> str:
    if isinstance(value, int):
        return "number " + str(value + 1)
    return "text " + value.upper()


def parse(text: str) -> int | float | None:
    if text == "":
        return None
    if "." in text:
        return float(text)
    return int(text)


values: list[int | str] = [1, "a", 22, "bc"]
print(values, len(values), values[1])
for value in values:
    print(describe(value), value, repr(value), type(value).__name__)
print(parse("3"), parse("2.5"), parse(""))
mixed: dict[str, int | float | str] = {"a": 1, "b": 2.5, "c": "x"}
print(mixed, mixed["b"])
keys: set[int | str] = {1, "1", 1}
print(len(keys), 1 in values, "a" in values)
size: int | str = 4
size = "four"
print(size, bool(size), str(size))
