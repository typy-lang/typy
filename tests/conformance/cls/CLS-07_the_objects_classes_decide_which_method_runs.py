# typy-test: CLS-07
# typy-expect: accepted
class B:
    def __eq__(self, other: "B") -> bool:
        return True


class V(B):
    def __eq__(self, other: "B") -> bool:
        return False


class W(V):
    def __eq__(self, other: "B") -> bool:
        return True


left: V = V()
right: B = W()
print(left == right, right == left, left == B(), B() == left)
