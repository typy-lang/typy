# typy-test: CLS-25
# typy-expect: accepted
class Tool:
    def __init__(self, name: str) -> None:
        self.name = name

    @staticmethod
    def helper(n: int) -> int:
        return n * 2


def make(name: str) -> Tool:
    print("make", name)
    return Tool(name)


tools = [Tool("a"), Tool("b")]
print(make("x").helper(3), tools[1].helper(4), Tool("c").helper(5))
