# typy-test: FUN-13
# typy-expect: rejected E0201 at 7:7
def total(*numbers: int) -> int:
    return sum(numbers)


add = total
