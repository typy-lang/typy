# typy-test: GEN-06
# typy-expect: rejected E0201 at 3:11
def count[*Ts](value: int) -> int:
    return value
