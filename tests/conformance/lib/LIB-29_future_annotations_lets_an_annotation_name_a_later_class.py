# typy-test: LIB-29, TYP-12
# typy-expect: accepted
"""`from __future__ import annotations` defers every annotation of the module, so one may
name a class before its `class` statement runs, under any Python floor."""

from __future__ import annotations, generator_stop


class Node:
    def __init__(self, value: int, next: Node | None) -> None:
        self.value = value
        self.next = next

    def last(self) -> Leaf:
        return Leaf(self.value)


class Leaf:
    def __init__(self, value: int) -> None:
        self.value = value


def total(node: Node | None) -> int:
    result = 0
    while node is not None:
        result += node.value
        node = node.next
    return result


chain = Node(1, Node(2, Node(3, None)))
print(total(chain), chain.last().value, type(chain.last()).__name__)
