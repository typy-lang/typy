# typy-test: EXP-18
# typy-expect: rejected E0201 at 5:15
def running_totals() -> None:
    total = 0
    values = (total := total + v for v in range(3))
    print(list(values))
