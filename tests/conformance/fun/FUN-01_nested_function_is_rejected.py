# typy-test: FUN-01
# typy-expect: rejected E0201 at 4:5
def outer() -> int:
    def inner() -> int:
        return 1
    return inner()
