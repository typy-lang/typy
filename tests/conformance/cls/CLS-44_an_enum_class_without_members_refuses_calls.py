# typy-test: CLS-44
# typy-expect: accepted
# Calling an enum class with no members raises TypeError, as EnumType.__call__ does.
from enum import Enum, Flag, IntEnum
class E(Enum):
    pass
class F(Flag):
    pass
class I(IntEnum):
    pass
try:
    E(1)
except TypeError as e:
    print(e)
try:
    F(0)
except TypeError as e:
    print(e)
try:
    I(2)
except TypeError as e:
    print(e)
print(list(E), len(F), list(I))
