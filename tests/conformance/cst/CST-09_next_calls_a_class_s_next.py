# typy-test: CST-09, CST-08
# typy-expect: accepted
class Countdown:
    def __init__(self, start: int) -> None:
        self.current = start

    def __iter__(self) -> "Countdown":
        return self

    def __next__(self) -> int:
        if self.current <= 0:
            raise StopIteration
        self.current -= 1
        return self.current + 1


countdown = Countdown(2)
same = iter(countdown)
print(same is countdown, next(countdown), next(same), next(countdown, 0), next(countdown, None))
try:
    next(countdown)
except StopIteration as error:
    print("done", repr(error))
