# typy-test: CST-05, EXC-04, STD-05
# typy-expect: accepted
class Failing:
    def __enter__(self) -> None:
        return None

    def __exit__(self, kind, error, trace) -> None:
        raise RuntimeError("exit failed")


with Failing():
    raise ValueError("body failed")
