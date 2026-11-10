# typy-test: CLS-07
# typy-expect: rejected E0201 at 4:9
class Money:
    def __eq__(self, other: "Money") -> bool:
        return True
