# typy-test: MEM-04
# typy-expect: accepted
class Box:
    def __init__(self, text: str) -> None:
        self.text = text
        self.items = [text, text + "!"]


current = Box("first" * 3)
table = {"key": "value" * 3}


def replace() -> str:
    # The old box, its text and its list are freed here unless an operand still holds them.
    global current
    current = Box("second" * 3)
    return "?"


def forget() -> str:
    table.clear()
    return "#"


def operands() -> None:
    print(current.text + replace(), current.text == replace(), current.text < replace())
    print(f"{current.text}{replace()}", (current.text, replace()), {current.text: replace()})
    print(current.items[len(replace())], current.text[len(replace()) - 1])
    print(table["key"] + forget())


def calls() -> None:
    print(current.items.count(replace()), current.text.startswith(replace()))
    print(current.text, replace())
    print(max(current.text, replace()), current.text[0 : len(replace())])


def targets() -> None:
    current.items[len(replace()) - 1] = "x"
    print(current.items)
    current.items[0] += replace()
    print(current.items)
    del current.items[len(replace()) - 1]
    print(current.items)


operands()
calls()
targets()
