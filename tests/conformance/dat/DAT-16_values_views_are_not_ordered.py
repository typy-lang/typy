# typy-test: DAT-16, EXP-06
# typy-expect: rejected E0407 at 4:23
ages = {"a": 1}
print(ages.values() < ages.values())
