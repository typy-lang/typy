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
print(sys.version_info >= (3, 12), sys.version_info < (4,), (3, 11) < sys.version_info)
print(sys.version_info.major, sys.version_info[0], sys.version_info[-2])
print(sys.version_info == (3, 12, 0, 0), sys.version_info != (2,), sys.version_info.serial)
print(sys.version.split()[0].startswith("3."), sys.hexversion >= 0x030C0000)
