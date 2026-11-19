# typy-test: FUN-04, STD-04
# typy-expect: rejected E0202 at 10:26
def outer() -> int:
    x = 1

    def read() -> int:
        return x
    try:
        raise ValueError("v")
    except ValueError as x:
        pass
    return read()
