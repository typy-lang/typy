# typy-test: CLS-01
# typy-expect: rejected E0306 at 4:32
class Tree:
    def __init__(self, parent: Tree | None) -> None:
        self.parent = parent
