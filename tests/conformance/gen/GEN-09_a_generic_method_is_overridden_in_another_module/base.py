class Visitor:
    def visit[T](self, value: T) -> list[T]:
        return [value]


def run(visitor: Visitor) -> None:
    print(visitor.visit(1), visitor.visit("a"))
