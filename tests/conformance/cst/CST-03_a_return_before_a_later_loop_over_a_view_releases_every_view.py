# typy-test: CST-03
# typy-expect: accepted
def first_key(table: dict[str, int], early: bool) -> str:
    if early:
        for key, value in table.items():
            return key
        return ""
    for key in table.keys():
        print(key)
    return "late"


def counted(table: dict[str, int]) -> int:
    pairs = {key: value for key, value in table.items() if value > 1}
    if not pairs:
        return 0
    return len({key: value for key, value in table.items()})


print(first_key({"a": 1, "b": 2}, True), first_key({"c": 3}, False))
print(counted({"a": 1}), counted({"a": 1, "b": 2}))
