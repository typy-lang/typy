# typy-test: FUN-13
# typy-expect: rejected E0203 at 7:7
def total(*numbers: int) -> int:
    return sum(numbers)


add = total
