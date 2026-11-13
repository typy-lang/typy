# typy-test: TYP-16
# typy-expect: rejected E0201 at 5:9
values: list[int | str] = [1, "a"]
first = 0
first = values[0] = 2
