# typy-test: SMT-10
# typy-expect: accepted
type Score = int
type Scores = list[Score]
type Table = dict[str, Scores]
type Found = Node | None


def best(scores: Scores) -> Score:
    return max(scores)


class Node:
    def __init__(self, value: Score) -> None:
        self.value = value


def first(nodes: list[Node]) -> Found:
    return nodes[0] if nodes else None


table: Table = {"ada": [3, 9], "alan": [7]}
for name, scores in table.items():
    print(name, best(scores))
node = first([Node(4)])
if node is not None:
    print(node.value)
print(first([]))
