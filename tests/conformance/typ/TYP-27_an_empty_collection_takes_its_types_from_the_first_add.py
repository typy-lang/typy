# typy-test: TYP-27
# typy-expect: accepted
def squares(n: int) -> list[int]:
    found = []
    for i in range(n):
        found.append(i * i)
    return found


def letters(words: list[str]) -> set[str]:
    found = set()
    for word in words:
        found.update(word)
    return found


def lengths(words: list[str]) -> dict[str, int]:
    found = {}
    for word in words:
        found[word] = len(word)
    return found


def table(n: int):
    rows = []
    for i in range(n):
        row = []
        for j in range(n):
            row.append(i * j)
        rows.append(row)
    return rows


print(squares(5))
print(sorted(letters(["ab", "bc"])))
print(lengths(["one", "three"]))
print(table(3))
