# typy-test: CST-10, CST-05
# typy-expect: accepted
from types import TracebackType


class Quiet:
    def __enter__(self) -> "Quiet":
        return self

    def __exit__(
        self,
        kind: type[BaseException] | None,
        error: BaseException | None,
        trace: TracebackType | None,
    ) -> bool:
        print("leaving", kind is None, trace is None)
        return True


with Quiet():
    print("inside")
with Quiet():
    raise ValueError("hidden")
print("after")
