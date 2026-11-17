# typy-test: GEN-09, GEN-07
# typy-expect: accepted
from base import Visitor, run


class Doubler(Visitor):
    def visit[T](self, value: T) -> list[T]:
        return [value, value]


run(Doubler())
v: Visitor = Doubler()
print(v.visit(1.5))
