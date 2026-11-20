# typy-test: EXP-17
# typy-expect: accepted
import sys


class Word:
    def __init__(self, text: str) -> None:
        self.text = text

    def __str__(self) -> str:
        return self.text.upper()


nothing: list[int] = []
print(*[1, 2, 3])
print(*nothing)
print(*nothing, *[4], sep="-")
print(0, *nothing, 5, *range(3), sep=", ", end=".\n")
print(*"abc", *(1.5, 2.5))
print(*[Word("a"), Word("b")], file=sys.stderr)
print("x", *[Word("y")])
