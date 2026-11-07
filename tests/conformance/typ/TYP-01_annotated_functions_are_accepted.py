# typy-test: TYP-01
# typy-expect: accepted
def scale(value: float, factor: float = 2.0) -> float:
    return value * factor


print(scale(1.5), scale(1.5, 3.0))
