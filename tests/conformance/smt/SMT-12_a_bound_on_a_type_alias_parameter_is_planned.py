# typy-test: SMT-12
# typy-expect: rejected E0201 at 3:11
type Some[T: int] = list[T]
