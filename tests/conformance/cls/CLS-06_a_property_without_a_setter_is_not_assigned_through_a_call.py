# typy-test: CLS-06
# typy-expect: rejected E0414 at 13:1
class Gauge:
    @property
    def level(self) -> int:
        return 1


def gauge() -> Gauge:
    return Gauge()


gauge().level = 2
