# typy-test: GEN-09, CLS-04, CLS-14
# typy-expect: accepted
class Visitor:
    def visit[T](self, value: T) -> list[T]:
        print("base visit", value)
        return [value]

    def name(self) -> str:
        return "visitor"


class Doubler(Visitor):
    def visit[T](self, value: T) -> list[T]:
        print("double visit", value)
        return [value, value]


class Loud(Doubler):
    def visit[U](self, value: U) -> list[U]:
        found = super().visit(value)
        print("loud", len(found))
        return found


def run(visitor: Visitor) -> None:
    print(visitor.visit(1), visitor.visit("a"))


run(Visitor())
run(Doubler())
run(Loud())
items: list[Visitor] = [Visitor(), Doubler(), Loud()]
print([each.visit(2.5) for each in items])
print(Loud().visit(True))
