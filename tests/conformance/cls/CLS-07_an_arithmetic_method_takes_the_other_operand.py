# typy-test: CLS-07
# typy-expect: rejected E0404 at 4:9
class Money:
    def __add__(self) -> "Money":
        return self
