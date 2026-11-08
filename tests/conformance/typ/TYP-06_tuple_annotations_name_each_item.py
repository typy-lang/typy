# typy-test: TYP-06
# typy-expect: accepted
def swap(pair: tuple[int, str]) -> tuple[str, int]:
    a, b = pair
    return b, a


def stats(x: float, y: float) -> tuple[float, float, bool]:
    return (x + y, x * y, x < y)


empty: tuple[()] = ()
pair: tuple[int, str] = (3, "three")
print(swap(pair), stats(1.5, 2.0), empty)
