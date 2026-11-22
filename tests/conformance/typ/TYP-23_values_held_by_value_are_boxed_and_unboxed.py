# typy-test: TYP-23
# typy-expect: accepted
def describe(value: object) -> str:
    if isinstance(value, str):
        return "str " + value.upper()
    if isinstance(value, int):
        return f"int {value}"
    return "other " + str(value)


items: list[object] = [1, "a", 2.5, None, True, (1, "b")]
for item in items:
    print(describe(item), item == 1, repr(item))
print(items)
