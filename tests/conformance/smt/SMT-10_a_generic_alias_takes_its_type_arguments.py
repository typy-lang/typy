# typy-test: SMT-10, GEN-01
# typy-expect: accepted
type Pair[T] = tuple[T, T]
type Rows[T] = list[Pair[T]]
type Table[K, V] = dict[K, list[V]]


def swap[T](pair: Pair[T]) -> Pair[T]:
    return (pair[1], pair[0])


rows: Rows[str] = [("a", "b")]
counts: Table[str, int] = {"x": [1, 2]}
print(swap((1, 2)), swap(rows[0]), counts)
