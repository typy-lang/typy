# typy-test: LIB-29, STD-01
# typy-expect: rejected E0103 at 4:1
import sys
from __future__ import annotations

print(sys.argv[1:])
