# typy-test: CST-08, STD-05
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


class WordIterator:
    def __init__(self, words: list[str]) -> None:
        self.words = words
        self.index = 0

    def __iter__(self) -> "WordIterator":
        return self

    def __next__(self) -> str:
        if self.index >= len(self.words):
            raise StopIteration()
        word = self.words[self.index]
        self.index += 1
        return word


class Sentence:
    def __init__(self, text: str) -> None:
        self.words = text.split()

    def __iter__(self) -> WordIterator:
        return WordIterator(self.words)


class Faulty:
    def __iter__(self) -> "Faulty":
        return self

    def __next__(self) -> int:
        raise ValueError("broken")


for n in Countdown(3):
    print(n)
else:
    print("lift off")
sentence = Sentence("the quick brown fox")
for word in sentence:
    if word == "brown":
        break
    print(word)
print([w.upper() for w in sentence])
for i, w in enumerate(Sentence("a b")):
    print(i, w)
for n, w in zip(Countdown(5), Sentence("x y z")):
    print(n, w)
for outer in Countdown(2):
    for inner in Countdown(outer):
        print(outer, inner)
try:
    for value in Faulty():
        print(value)
except ValueError as error:
    print("caught", error)
for value in Faulty():
    print(value)
