# typy-test: LIB-16
# typy-expect: accepted
import sys
from sys import version_info

info = sys.version_info
kept = [info, version_info]
print(info >= (3, 12), (3, 12) <= version_info, info[0], info.major, info.releaselevel)
print(len(info), info[:1], info is version_info, kept[1] is sys.version_info)
print(type(info).__name__, info == version_info, info != (3,))
major, minor, micro, level, serial = info
print(major, level, serial)
for item in info[:1]:
    print(item)
