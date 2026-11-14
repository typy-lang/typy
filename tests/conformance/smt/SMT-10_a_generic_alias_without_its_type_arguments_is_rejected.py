# typy-test: SMT-10
# typy-expect: rejected E0408 at 6:9
type Pair[T] = tuple[T, T]


points: Pair = (1, 2)
