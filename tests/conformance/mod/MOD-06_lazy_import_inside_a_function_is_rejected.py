# typy-test: MOD-06
# typy-expect: rejected E0103 at 7:5
# typy-python: 3.15


def load() -> None:
    lazy import json
