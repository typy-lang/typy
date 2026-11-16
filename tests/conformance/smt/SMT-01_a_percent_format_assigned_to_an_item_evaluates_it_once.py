# typy-test: SMT-01
# typy-expect: accepted
def pick() -> int:
    print("pick")
    return 0


messages = ["%s and %s"]
values: tuple[str, ...] = ("a", "b")
messages[pick()] %= values
print(messages)


class Box:
    def __init__(self) -> None:
        self.text = "<%s>"


def box() -> Box:
    print("box")
    return made


made = Box()
box().text %= values[:1]
print(made.text)
