# typy-test: DAT-16, CST-08
# typy-expect: accepted
class Countdown:
    def __init__(self, start: int) -> None:
        self.current = start

    def __iter__(self) -> "Countdown":
        print("iter")
        return self

    def __next__(self) -> tuple[str, int]:
        if self.current == 0:
            raise StopIteration
        self.current -= 1
        return (f"k{self.current}", self.current)


def note(text: str, value: int) -> int:
    print("evaluate", text)
    return value


print(dict(a=1, b=2), dict(b=1, a=2))
base = {"x": 1, "a": 0}
print(dict(base, a=5, z=6), base)
print(dict([("p", 1), ("q", 2)], q=9, r=note("r", 3)))
print(dict(zip(["m", "n"], [1, 2]), n=note("n", 7)))
print(dict(Countdown(2), k0=note("k0", 100)))
grow = [("a", 1)]
print(dict(grow, b=len(grow)))
scores: dict[str, float] = dict(ada=1.5, alan=2.5)
print(scores)
