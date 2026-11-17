# typy-test: CLS-26
# typy-expect: accepted
class A:
    def __init__(self) -> None:
        self.a = 1

    def name(self) -> str:
        return "A"


class B(A):
    def __init__(self) -> None:
        super(B, self).__init__()
        self.b = 2

    def name(self) -> str:
        return "B" + super(B, self).name()


class C(B):
    def __init__(self) -> None:
        super(B, self).__init__()
        self.b = 3
        self.c = 4

    def name(self) -> str:
        return "C" + super(B, self).name() + super().name()


c = C()
print(c.name(), c.a, c.b, c.c, B().name())
