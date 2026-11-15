# typy-test: GEN-03, GEN-01, GEN-02
# typy-expect: accepted
class Shape:
    def area(self) -> float:
        return 2.0


class Node[T]:
    def __init__(self, value: T, next: "Node[T] | None") -> None:
        self.value = value
        self.next = next

    def __repr__(self) -> str:
        return "Node(" + repr(self.value) + ")"

    def __eq__(self, other: "Node[T]") -> bool:
        return self.value == other.value

    def length(self) -> int:
        count = 1
        node = self.next
        while node is not None:
            count += 1
            node = node.next
        return count

    def values(self) -> list[T]:
        found: list[T] = [self.value]
        rest = self.next
        if rest is not None:
            found.extend(rest.values())
        return found


class Holder[S: Shape]:
    def __init__(self, shape: S) -> None:
        self.shape = shape

    def area(self) -> float:
        return self.shape.area() * 2


chain = Node(1, Node(2, Node(3, None)))
print(chain.length(), chain.values(), chain, [chain.next])
print(Node("a", None) == Node("a", None), Node("a", None) == Node("b", None))
print(Holder(Shape()).area())
words: dict[str, Node[str]] = {"x": Node("y", None)}
print(words, isinstance(words["x"], Node))
