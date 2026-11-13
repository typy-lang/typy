# typy-test: CLS-11, MOD-01
# typy-expect: accepted
import settings
from settings import Limits


class Strict(Limits):
    retries = 1


Limits.retries += 2
limits = Limits()
print(Limits.retries, settings.Limits.retries, limits.retries, limits.timeout)
print(Strict.retries, Strict().retries, Strict().timeout, Limits.timeout)
