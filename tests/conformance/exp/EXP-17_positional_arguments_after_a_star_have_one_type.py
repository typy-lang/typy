# typy-test: EXP-17
# typy-expect: rejected E0409 at 8:17
def label(count: int, size: int | None) -> str:
    return f"{count} {size}"


numbers = [1]
label(*numbers, 2.5)
