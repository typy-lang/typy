class Base:
    def __init__(self, value: int) -> None:
        self.value = value

    def __eq__(self, other: "Base") -> bool:
        return self.value == other.value
