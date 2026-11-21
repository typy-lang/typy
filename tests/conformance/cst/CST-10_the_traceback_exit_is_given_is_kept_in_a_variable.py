# typy-test: CST-10
# typy-expect: accepted
# typy-python: 3.14
from types import TracebackType


class Quiet:
    def __enter__(self) -> Quiet:
        return self

    def __exit__(
        self,
        kind: type[BaseException] | None,
        value: BaseException | None,
        trace: TracebackType | None,
    ) -> bool:
        if trace is not None:
            held: TracebackType = trace
            print(held is trace)
        return True


with Quiet():
    raise ValueError("x")
print("done")
