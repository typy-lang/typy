# typy-test: FUN-05
# typy-expect: rejected E0404 at 8:13
def double(value: int) -> int:
    return value * 2


twice = double
print(twice(value=2))
