# typy-test: GEN-12
# typy-expect: rejected E0201 at 3:7
class Failure[T](Exception):
    pass
