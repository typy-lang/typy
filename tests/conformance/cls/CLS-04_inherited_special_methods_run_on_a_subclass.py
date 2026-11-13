# typy-test: CLS-04
# typy-expect: accepted
class Countdown:
    def __init__(self, start: int) -> None:
        self.current = start

    def __iter__(self) -> "Countdown":
        return self

    def __next__(self) -> int:
        if self.current == 0:
            raise StopIteration
        self.current -= 1
        return self.current + 1

    def __enter__(self) -> "Countdown":
        print("enter", self.current)
        return self

    def __exit__(self, kind, error, trace) -> None:
        print("exit", self.current)

    def __str__(self) -> str:
        return "Countdown at " + str(self.current)


class Launch(Countdown):
    pass


launch = Launch(3)
print(launch)
for step in launch:
    print(step)
with Launch(2) as counting:
    print(list(counting), counting)
print(str(Launch(5)), repr(type(launch).__name__))
