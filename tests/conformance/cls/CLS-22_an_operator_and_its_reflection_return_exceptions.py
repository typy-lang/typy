# typy-test: CLS-22
# typy-expect: accepted
# typy-python: 3.14
class Left:
    def __add__(self, other: Right) -> ValueError:
        return ValueError("left")


class Right:
    def __radd__(self, other: Left) -> KeyError:
        return KeyError("right")


print(repr(Left() + Right()))
