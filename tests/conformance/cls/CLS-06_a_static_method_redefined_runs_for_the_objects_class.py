# typy-test: CLS-06
# typy-expect: accepted
class Base:
    @staticmethod
    def kind() -> str:
        return "base"

    def describe(self, x: int) -> str:
        return f"Base.describe {x}"

    @staticmethod
    def make(x: int) -> int:
        return x


class Child(Base):
    @staticmethod
    def kind() -> str:
        return "child"

    @staticmethod
    def describe(x: int) -> str:
        return f"Child.describe static {x}"

    def make(self, x: int) -> int:
        return x * 100


class Grand(Child):
    @staticmethod
    def kind() -> str:
        return "grand"


def show(item: Base) -> None:
    print(item.kind(), item.describe(1), item.make(2), Base.kind(), Base.make(3))


for each in [Base(), Child(), Grand()]:
    show(each)
child: Child = Grand()
print(child.kind(), Child.kind(), child.describe(5), Child.describe(6), child.make(7))
