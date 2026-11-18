# typy-test: FUN-04
# typy-expect: rejected E0202 at 6:5
def outer() -> None:
    def helper() -> int:
        return 1
    helper = 3
