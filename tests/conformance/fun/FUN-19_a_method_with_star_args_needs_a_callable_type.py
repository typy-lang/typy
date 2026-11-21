# typy-test: FUN-19
# typy-expect: rejected E0203 at 10:15
class Meter:
    def total(self, *values: int) -> int:
        return sum(values)


meter = Meter()
print(meter.total(1, 2))
loose = meter.total
