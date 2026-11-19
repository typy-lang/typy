# typy-test: SMT-09, STD-01
# typy-expect: rejected E0103 at 5:18
def outer() -> None:
    def inner() -> None:
        nonlocal missing
        missing = 1
    inner()
