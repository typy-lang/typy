# typy-test: SMT-09, STD-04
# typy-expect: rejected E0202 at 10:30
def outer() -> str:
    error = ValueError("first")

    def inner() -> None:
        nonlocal error
        try:
            raise ValueError("second")
        except ValueError as error:
            pass
    inner()
    return str(error)
