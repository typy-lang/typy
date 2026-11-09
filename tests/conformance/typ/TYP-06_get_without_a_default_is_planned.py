# typy-test: TYP-06
# typy-expect: rejected E0201 at 4:7
ages = {"a": 1}
print(ages.get("b"))
