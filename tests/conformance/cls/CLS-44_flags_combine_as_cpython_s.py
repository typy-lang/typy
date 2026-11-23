# typy-test: CLS-44
# typy-expect: accepted
# Flag and IntFlag members combine into composites, made once each, named by their members
# in definition order; a Flag refuses bits it does not define, and an IntFlag keeps them.
from enum import Flag, IntFlag, auto


class Perm(Flag):
    X = 4
    R = 1
    W = auto()


class Bits(IntFlag):
    A = 1
    B = 2


rw = Perm.R | Perm.W
print(repr(rw), rw, rw.name, rw.value, list(rw), len(rw), Perm.R in rw, Perm.X in rw)
print(repr(~Perm.R), repr(rw & Perm.R), repr(rw ^ Perm.W), rw is (Perm.W | Perm.R))
print(repr(Perm(0)), Perm(0), Perm(0).name, bool(Perm(0)), list(Perm), len(Perm))
print(repr(Bits.A | 4), Bits.A | 4, repr(~Bits.A), Bits.A + 1, repr(Bits(8)), Bits.B == 2)
for value in (-3, 5, 13, 16):
    try:
        print(repr(Perm(value)))
    except ValueError as error:
        print(error)
    print(repr(Bits(value)), len(Bits(value)), list(Bits(value)))
