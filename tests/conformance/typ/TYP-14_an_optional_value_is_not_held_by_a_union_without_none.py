# typy-test: TYP-14
# typy-expect: rejected E0402 at 4:23
def widened(value: int | None) -> None:
    held: int | str = value
