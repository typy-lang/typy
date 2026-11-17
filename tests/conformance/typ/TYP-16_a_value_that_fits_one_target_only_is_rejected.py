# typy-test: TYP-16
# typy-expect: rejected E0402 at 5:9
names: list[str] = ["a"]
count = 0
count = names[0] = 1
