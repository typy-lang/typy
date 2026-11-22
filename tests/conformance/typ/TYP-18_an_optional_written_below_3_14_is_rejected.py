# typy-test: TYP-18
# typy-expect: rejected E0202 at 6:7
from typing import Optional

print(Optional[int] == int | None)
print(Optional[int])
