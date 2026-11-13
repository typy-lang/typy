# typy-test: TYP-14
# typy-expect: accepted
values: list[int | str] = [1, "a"]
values.append(2)
values.insert(0, "z")
values[1] = 5
values.remove("a")
values.extend([7, "q"])
print(values, values.count(5), values.index(2))
keys: set[int | str] = {1}
keys.add("b")
keys.discard(1)
print(keys, "b" in keys)
table: dict[int | str, int | str] = {1: "one"}
table["two"] = 2
table[3] = "three"
print(table, table[1], table.get("two"), table.get(9, "none"), 3 in table)
print(table.pop(3), table.setdefault(4, "four"), table)
