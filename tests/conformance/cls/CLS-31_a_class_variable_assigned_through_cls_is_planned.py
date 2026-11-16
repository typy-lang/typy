# typy-test: CLS-31
# typy-expect: rejected E0201 at 8:9
class Counter:
    count = 0

    @classmethod
    def bump(cls) -> None:
        cls.count = cls.count + 1


class Ticket(Counter):
    pass
