# typy-test: MEM-09
# typy-expect: accepted
class Box:
    def __init__(self, label: str) -> None:
        self.label = label
        self.items: list[str] = []


def build(count: int) -> list[list[str]]:
    rows: list[list[str]] = []
    for index in range(count):
        row = [str(index), "x" * index]
        name = "row" + str(index)
        row.append(name)
        rows.append(row)
    return rows


def reused(count: int) -> list[str]:
    parts: list[str] = []
    text = "start"
    for index in range(count):
        parts.append(text)
        text = text + str(index)
    parts.append(text)
    return parts


def loop_read_next_iteration(count: int) -> list[str]:
    seen: list[str] = []
    item = "a"
    while len(seen) < count:
        seen.append(item)
        if len(seen) == 2:
            continue
    return seen


def in_handlers(flag: bool) -> list[str]:
    out: list[str] = []
    value = "kept for the handler"
    try:
        out.append(value)
        if flag:
            raise ValueError("boom")
    except ValueError:
        out.append(value)
    finally:
        out.append(value + "!")
    return out


def with_finally() -> str:
    first = "one"
    second = "two"
    try:
        pair = [first, second]
        return pair[0]
    finally:
        print("finally sees", second)


def entries() -> dict[str, list[str]]:
    table: dict[str, list[str]] = {}
    for word in ["a", "bb", "a", "ccc"]:
        bucket = [word, word.upper()]
        key = word
        table[key] = bucket
    return table


def sets() -> set[str]:
    found: set[str] = set()
    for word in ["x", "y", "x"]:
        copy = word * 2
        found.add(copy)
    return found


def tuples() -> list[tuple[str, int]]:
    out: list[tuple[str, int]] = []
    for index in range(3):
        label = "t" + str(index)
        pair = (label, index)
        out.append(pair)
    return out


def boxes() -> list[str]:
    box = Box("b")
    for index in range(3):
        word = "w" + str(index)
        box.items.append(word)
    label = box.label + "!"
    box.label = label
    return box.items + [box.label]


def comprehensions() -> list[list[str]]:
    base = "c"
    return [[base, str(index)] for index in range(3)]


def twice() -> list[str]:
    text = "twice"
    pair = [text, text]
    return pair


def inserted() -> list[str]:
    out = ["z"]
    for index in range(3):
        word = str(index)
        out.insert(0, word)
    return out


def assign_item() -> list[str]:
    out = ["", "", ""]
    for index in range(3):
        word = "i" + str(index)
        out[index] = word
    try:
        late = "late"
        out[10] = late
    except IndexError as error:
        print("IndexError", error)
    return out


def broken(limit: int) -> list[str]:
    out: list[str] = []
    for index in range(10):
        word = "b" + str(index)
        if index == limit:
            out.append(word)
            break
        out.append(word)
    return out


print(build(3))
print(reused(3))
print(loop_read_next_iteration(3))
print(in_handlers(True), in_handlers(False))
print(with_finally())
print(entries())
print(sorted(sets()))
print(tuples())
print(boxes())
print(comprehensions())
print(twice())
print(inserted())
print(assign_item())
print(broken(2))
