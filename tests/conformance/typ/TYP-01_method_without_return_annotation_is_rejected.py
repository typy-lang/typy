# typy-test: TYP-01
# typy-expect: rejected E0204 at 4:9
class Counter:
    def value(self):
        return 1


print(Counter().value())
