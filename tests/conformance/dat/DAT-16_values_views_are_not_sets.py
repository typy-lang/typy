# typy-test: DAT-16, EXP-02
# typy-expect: rejected E0401 at 4:7
ages = {"a": 1}
print(ages.values() | {1})
