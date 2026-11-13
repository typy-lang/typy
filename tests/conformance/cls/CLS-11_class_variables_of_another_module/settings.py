class Limits:
    retries = 3
    timeout: float = 1.5

    def __init__(self) -> None:
        self.name = "limits"
