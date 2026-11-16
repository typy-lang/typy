# typy-test: CLS-36
# typy-expect: rejected E0201 at 10:13
class A:
    def m(self) -> int:
        return 1


class B(A):
    def m(self) -> int:
        s = super()
        return s.m()


print(B().m())
