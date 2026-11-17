# typy-test: CST-06
# typy-expect: accepted
def len(value: list[int]) -> int:
    return 99


def first_two(values: list[int]) -> str:
    match values:
        case [a, b]:
            return f"two {a} {b}"
        case [a, b, *_]:
            return f"more {a} {b}"
        case [x] | [x, _, _, _, _]:
            return f"x {x}"
    return "other"


def counts(table: dict[int, list[str]]) -> str:
    match table:
        case {1: [only], **rest}:
            return f"{only} {rest}"
        case {**everything}:
            return f"all {everything}"


total = 0
for round_ in range(3):
    match [round_, round_ + 1]:
        case [0, b]:
            total += b
        case [a, b] if a == b - 1:
            total += a * 10
print(first_two([1, 2]), first_two([1, 2, 3]), first_two([4]), first_two([]))
print(counts({1: ["a"], 2: ["b"]}), counts({1: ["a", "b"]}), counts({}))
print(total, len([1]))
