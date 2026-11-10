# typy-test: MEM-04
# typy-expect: accepted
class Box:
    def __init__(self) -> None:
        self.items: list[str] = ["a", "b"]
        self.name = "box"


def replace(items: list[str], box: Box) -> str:
    box.items = ["z"]
    box.name = "other"
    return items[0] + items[1] + box.items[0]


def clear_then_read(word: str, words: list[str]) -> str:
    words.clear()
    return word + "!"


def drop_then_read(value: list[int], table: dict[str, list[int]]) -> int:
    table.clear()
    return len(value) + value[0]


def first(items: list[str]) -> str:
    result = items[0]
    return result


box = Box()
print(replace(box.items, box), box.items, box.name)
words = ["h" + "i", "there"]
print(clear_then_read(words[0], words), words)
table = {"k": [4, 5, 6]}
print(drop_then_read(table["k"], table), table)
print(first(["x" + "y", "z"]), first(box.items))
