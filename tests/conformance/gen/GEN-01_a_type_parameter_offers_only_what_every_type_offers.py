# typy-test: GEN-01
# typy-expect: rejected E0412 at 4:18
def shout[T](value: T) -> str:
    return value.upper()
