# typy-test: SMT-09, TYP-13
# typy-expect: rejected E0401 at 11:16
def total() -> int:
    value: int | None = 1

    def clear() -> None:
        nonlocal value
        value = None
    if value is not None:
        clear()
        return value + 1
    return 0
