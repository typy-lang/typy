# typy-test: LIB-13
# typy-expect: accepted
import sys
from sys import maxsize, byteorder

print(maxsize, sys.maxunicode, byteorder, sys.platform, maxsize == 2**63 - 1)
print(sys.getrecursionlimit())
sys.setrecursionlimit(5000)
print(sys.getrecursionlimit())
for bad in [0, -5, 2**31, 2**80]:
    try:
        sys.setrecursionlimit(bad)
    except ValueError as error:
        print("ValueError", error)
    except OverflowError as error:
        print("OverflowError", error)
print(sys.getrecursionlimit(), repr(sys.stdin))
