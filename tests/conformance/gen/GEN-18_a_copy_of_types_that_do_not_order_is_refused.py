# typy-test: GEN-18
# typy-expect: rejected E0407 at 5:20
# A copy whose type arguments do not order refuses the comparison its body makes.
def before[T](first: T, second: T) -> bool:
    return first < second


print(before({"a": 1}, {"b": 2}))
