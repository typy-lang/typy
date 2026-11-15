# typy-test: LIB-17
# typy-expect: rejected E0408 at 4:17
from typing import Optional
value: Optional[int, str] = 1
