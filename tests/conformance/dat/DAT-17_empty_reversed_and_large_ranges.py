# typy-test: DAT-17, CST-03, CST-09
# typy-expect: accepted
class Span:
    def __init__(self, values: range) -> None:
        self.values = values


def pick(flag: bool) -> range:
    if flag:
        return range(3)
    return range(10, 0, -4)


def main() -> None:
    for r in [range(0), range(5, 0), range(10, 0, -3), range(-3, 4, 2), range(7, 7, -1)]:
        print(r, list(r), list(reversed(r)), len(r), bool(r), r[::-2], r[1:-1])
    big = range(-(2**65), 2**65, 2**64)
    print(big, len(big), list(big), list(reversed(big)), big[1:], big[::-1])
    print(type(iter(range(2**62, 2**62 + 1))).__name__)
    print(type(iter(range(2**63 - 2, 2**63 - 1))).__name__)
    print(type(iter(range(2**63, 2**63 + 1))).__name__)
    pair = (range(2), "x")
    print(pair, pair[0][1])
    seen = {range(0), range(1, 1), range(3)}
    print(len(seen), range(0, 3) in seen)
    span = Span(pick(False))
    print(span.values, sum(span.values), max(span.values), min(span.values), sorted(span.values))
    span.values = pick(True)
    print(span.values, span.values == range(3))
    it = iter(range(10**20, 10**20 + 2))
    print(next(it), next(it), next(it, 0))
    count: int = 0
    for value in range(-10**19, -10**19 + 3):
        count += value
    print(count)
    print(f"{range(4)}", str(range(1, 2)), [range(4)][0][-4])
    print(range(4)[-5:99], range(0, 100, 7)[2:9:3], 3 in range(10**30), 10**30 in range(10**30))
    print(range(3) == range(3), range(1, 3) == range(0, 3), range(3) != range(4))


main()
