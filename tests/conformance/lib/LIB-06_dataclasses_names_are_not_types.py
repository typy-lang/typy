# typy-test: LIB-06
# typy-expect: rejected E0408 at 6:10
from dataclasses import dataclass


def f(x: dataclass) -> None:
    pass
