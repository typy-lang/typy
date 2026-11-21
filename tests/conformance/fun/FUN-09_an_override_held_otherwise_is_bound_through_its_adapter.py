# typy-test: FUN-09
# typy-expect: accepted
from collections.abc import Callable


class Meter:
    def read(self, scale: int) -> float:
        return scale * 1.5


class Gauge(Meter):
    def read(self, scale: int | None) -> float:
        return 0.0 if scale is None else scale * 2.0


class Probe:
    def find(self, key: int) -> int | None:
        return None


class Index(Probe):
    def find(self, key: int) -> int:
        return key + 1


class Cache(Index):
    def find(self, key: int | None) -> int:
        return 7 if key is None else key * 10


def run(meter: Meter) -> float:
    method: Callable[[int], float] = meter.read
    return method(4)


print(run(Meter()), run(Gauge()), Gauge().read(None))
gauge = Gauge()
held: Meter = gauge
print(held.read == gauge.read, gauge.read == held.read, held.read != Gauge().read)
print(repr(held.read)[:27], repr(Meter().read)[:27])
for probe in [Probe(), Index(), Cache()]:
    method: Callable[[int], int | None] = probe.find
    print(method(3), method == probe.find)
cache = Cache()
base: Probe = cache
middle: Index = cache
print(base.find == cache.find, middle.find == base.find, base.find(2))
