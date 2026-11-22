# typy-test: TYP-18
# typy-expect: accepted
# typy-python: 3.15
type Score = int

print(list[Score], Score | None, dict[str, Score] == dict[str, Score])
