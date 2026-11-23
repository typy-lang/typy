# typy-test: TYP-01
# typy-expect: rejected E0204 at 5:9
class Counter:
    @staticmethod
    def value():
        return 1


print(Counter.value())
