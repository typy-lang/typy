# typy-test: TYP-18
# typy-expect: rejected E0202 at 5:7
type Pair[T] = tuple[T, T]

print(Pair[int])
