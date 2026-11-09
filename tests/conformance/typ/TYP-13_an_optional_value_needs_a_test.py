# typy-test: TYP-13
# typy-expect: rejected E0401 at 4:7
score: int | None = None
print(score + 1)
