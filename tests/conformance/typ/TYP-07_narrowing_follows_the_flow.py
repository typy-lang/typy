# typy-test: TYP-07
# typy-expect: accepted
class Node:
    def __init__(self, name: str) -> None:
        self.name = name


class Leaf(Node):
    def __init__(self, name: str, value: int) -> None:
        super().__init__(name)
        self.value = value


class Branch(Node):
    def __init__(self, name: str, children: list[Node]) -> None:
        super().__init__(name)
        self.children = children


def total(node: Node) -> int:
    if isinstance(node, Leaf):
        return node.value
    if not isinstance(node, Branch):
        raise TypeError("unknown node " + node.name)
    result = 0
    for child in node.children:
        result += total(child)
    return result


def first_leaf(node: Node | None) -> str:
    while node is not None:
        if isinstance(node, Leaf):
            return node.name
        if isinstance(node, Branch) and len(node.children) > 0:
            node = node.children[0]
        else:
            node = None
    return "none"


tree = Branch("root", [Leaf("a", 1), Branch("b", [Leaf("c", 2), Leaf("d", 3)])])
print(total(tree), first_leaf(tree), first_leaf(None), first_leaf(Node("lone")))
try:
    total(Node("odd"))
except TypeError as error:
    print("error:", error)
