# typy-test: EXP-02
# typy-expect: rejected E0401 at 7:5
limit = 10


def check() -> None:
    limit()
