# typy-test: DAT-12
# typy-expect: accepted
def window(start: int | None) -> list[int]:
    part = slice(start, 2)
    return [1, 2, 3][part]


print(window(1), window(None))
