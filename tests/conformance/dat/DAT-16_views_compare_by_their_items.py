# typy-test: DAT-16
# typy-expect: accepted
ages = {"ada": 36, "alan": 41, "grace": 85}
other = {"alan": 41, "grace": 80, "linus": 54}
names = {"ada", "edsger", "grace"}
keys = ages.keys()
print(keys == {"ada", "alan", "grace"}, keys != names, keys < {"ada", "alan", "grace", "x"})
print(keys <= keys, keys > {"ada"}, keys >= other.keys(), {"ada"} < keys, names == keys)
items = ages.items()
print(items == {("ada", 36), ("alan", 41), ("grace", 85)}, items <= other.items())
print(ages.keys() == ages.keys(), items == dict(ages).items(), items > {("ada", 36)})
lists = {"a": [1], "b": [2]}
print(lists.items() == {"a": [1], "b": [2]}.items(), lists.items() < lists.items())
print(lists.items() >= {"a": [1]}.items(), lists.items() != {"a": [3], "b": [2]}.items())
values = ages.values()
print(values == values, values == ages.values(), values != ages.values())
