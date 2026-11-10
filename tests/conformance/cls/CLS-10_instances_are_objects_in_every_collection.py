# typy-test: CLS-10
# typy-expect: accepted
class Tag:
    def __init__(self, name: str) -> None:
        self.name = name


red = Tag("red")
blue = Tag("blue")
seen = {red, blue, red}
counts = {red: 1, blue: 2}
counts[red] += 5
print(len(seen), red in seen, Tag("red") in seen, counts[red], counts[blue])
tags = [red, blue, red]
print(tags.count(red), tags.index(blue), red in tags, (red, 1) == (red, 1))
text = repr(Tag("x"))
print(text.startswith("<__main__.Tag object at 0x"), text.endswith(">"))
