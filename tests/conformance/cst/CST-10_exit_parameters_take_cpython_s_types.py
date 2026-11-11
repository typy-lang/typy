# typy-test: CST-10, CST-05
# typy-expect: accepted
class Timer:
    def __enter__(self) -> "Timer":
        return self

    def __exit__(
        self, kind: type[BaseException] | None, error: BaseException | None, trace
    ) -> bool | None:
        if kind is not None:
            print("leaving with", kind.__name__, error, trace is not None)
            return kind is KeyError
        print("leaving cleanly", trace is None)
        return None


with Timer():
    print("timed")
with Timer():
    raise KeyError("ignored")
with Timer():
    raise ValueError("kept")
