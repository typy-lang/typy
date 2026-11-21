# typy-test: LIB-02
# typy-expect: accepted
empty: list[float] = []
print(max([1.5, 2.5], default=0.0), max(empty, default=0.5))
