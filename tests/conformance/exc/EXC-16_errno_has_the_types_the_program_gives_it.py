# typy-test: EXC-16
# typy-expect: rejected E0402 at 10:19
def fail(code: str) -> None:
    raise OSError(code, "failed")


try:
    fail("E42")
except OSError as error:
    number: int = error.errno
    print(number)
