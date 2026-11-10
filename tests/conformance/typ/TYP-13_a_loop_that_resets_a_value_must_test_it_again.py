# typy-test: TYP-13
# typy-expect: rejected E0401 at 5:11
last: int | None = 0
while True:
    print(last + 1)
    last = None
