# typy-test: LIB-04
# typy-expect: rejected E0202 at 4:11
import sys
print(sys.getrefcount(1))
