# typy-test: DAT-16
# typy-expect: rejected E0201 at 4:22
ages = {"a": 1}
print(ages.keys() == {"a"})
