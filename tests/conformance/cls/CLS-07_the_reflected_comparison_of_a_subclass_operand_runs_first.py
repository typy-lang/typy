# typy-test: CLS-07
# typy-expect: accepted
class A:
    def __init__(self, n: int) -> None:
        self.n = n

    def __lt__(self, other: "A") -> bool:
        print("A.lt")
        return self.n < other.n

    def __gt__(self, other: "A") -> bool:
        print("A.gt")
        return self.n > other.n


class B(A):
    pass


print(A(1) < B(2))
print(A(1) < A(2))
print(B(1) < A(2))
print(B(1) > B(2))
