# typy-test: TYP-26, FUN-07
# typy-expect: rejected E0204 at 3:5
def numbers(n: int):
    yield n


print(list(numbers(2)))
