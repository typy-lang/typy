# typy-test: MOD-11
# typy-expect: accepted
# typy-python: 3.15
import middle

print("main starts")
print(middle.double(4))
from middle import LIMIT

print(LIMIT, middle.LIMIT)
