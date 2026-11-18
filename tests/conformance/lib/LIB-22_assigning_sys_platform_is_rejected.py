# typy-test: LIB-22
# typy-expect: rejected E0402 at 5:1
import sys

sys.platform = "win32"
print(sys.platform)
