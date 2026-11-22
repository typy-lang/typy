# typy-test: EXP-22
# typy-expect: rejected E0201 at 9:15
class Failure(Exception):
    def __init__(self, code: int) -> None:
        self.code = code


codes = [1]
raise Failure(*codes)
