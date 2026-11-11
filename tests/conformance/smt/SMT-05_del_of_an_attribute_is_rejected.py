# typy-test: SMT-05
# typy-expect: rejected E0421 at 9:5
class Box:
    def __init__(self, value: int) -> None:
        self.value = value


box = Box(1)
del box.value
