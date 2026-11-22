# typy-test: EXP-22
# typy-expect: rejected E0409 at 9:8
class Pair(Exception):
    def __init__(self, name: str, *codes: int) -> None:
        self.name = name


codes = [1, 2]
pair = Pair("x", *codes)
