# typy-test: CST-08
# typy-expect: rejected E0411 at 8:13
class Box:
    def __init__(self, value: int) -> None:
        self.value = value


for item in Box(1):
    print(item)
