# typy-test: CLS-01
# typy-expect: rejected E0309 at 4:5
class Flag:
    def __init__(self, on: bool) -> None:
        if on:
            self.state = 1
