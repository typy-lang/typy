# typy-test: TYP-26
# typy-expect: rejected E0204 at 3:5
def countdown(n: int):
    if n == 0:
        return 0
    return countdown(n - 1)


print(countdown(3))
