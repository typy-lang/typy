# typy-test: EXP-08
# typy-expect: accepted
def finished(count: int) -> bool:
    return count > 2


f: int | None = None
g = finished(3)
h = finished(1)
print(g is True, f is None, h is False)
print(g is not True, h is not False, g is h, g is not h, (1 < 2) is True)
if finished(5) is True:
    print("done")
