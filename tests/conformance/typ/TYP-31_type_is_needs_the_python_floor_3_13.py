# typy-test: TYP-31
# typy-expect: rejected E0425 at 4:20
# TypeIs is in typing from Python 3.13, so CPython 3.12 raises ImportError.
from typing import TypeIs
