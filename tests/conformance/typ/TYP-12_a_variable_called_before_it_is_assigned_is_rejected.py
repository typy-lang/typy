# typy-test: TYP-12
# typy-expect: rejected E0306 at 6:14
def total() -> int:
    def double(value: int) -> int:
        return value * 2
    result = triple(2) + double(1)

    def triple(value: int) -> int:
        return value * 3
    return result
