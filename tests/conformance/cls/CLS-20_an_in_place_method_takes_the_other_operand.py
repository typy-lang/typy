# typy-test: CLS-20
# typy-expect: rejected E0404 at 4:9
class Money:
    def __iadd__(self) -> "Money":
        return self
