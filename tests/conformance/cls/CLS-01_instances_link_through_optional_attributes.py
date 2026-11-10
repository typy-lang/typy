# typy-test: CLS-01
# typy-expect: accepted
class Node:
    def __init__(self, value: int, next: "Node | None" = None) -> None:
        self.value = value
        self.next = next


class Stack:
    def __init__(self) -> None:
        self.top: Node | None = None
        self.size = 0

    def push(self, value: int) -> None:
        self.top = Node(value, self.top)
        self.size += 1

    def pop(self) -> int:
        top = self.top
        if top is None:
            return -1
        self.top = top.next
        self.size -= 1
        return top.value


stack = Stack()
for value in range(5):
    stack.push(value * value)
print(stack.pop(), stack.pop(), stack.size)
node = stack.top
total = 0
while node is not None:
    total += node.value
    node = node.next
print(total, stack.top is not None)
