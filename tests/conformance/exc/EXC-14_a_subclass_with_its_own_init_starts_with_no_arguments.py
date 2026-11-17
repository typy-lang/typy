# typy-test: EXC-14
# typy-expect: accepted
class Quiet(OSError):
    def __init__(self, text: str) -> None:
        self.text = text


class One(OSError):
    def __init__(self, text: str) -> None:
        super().__init__(text)


error = Quiet("q")
print(repr(error), str(error), error.args, error.errno, error.text)
other = One("one")
print(repr(other), other.args, other.errno)
