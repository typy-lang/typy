# typy-test: CLS-04
# typy-expect: rejected E0404 at 9:9
class Base:
    def run(self, steps: int) -> None:
        print(steps)


class Derived(Base):
    def run(self, *steps: int) -> None:
        print(steps)
