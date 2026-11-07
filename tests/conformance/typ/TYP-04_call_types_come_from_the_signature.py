# typy-test: TYP-04
# typy-expect: accepted
def half(n: int) -> float:
    return n / 2


result = half(5)
print(result + 0.25)
