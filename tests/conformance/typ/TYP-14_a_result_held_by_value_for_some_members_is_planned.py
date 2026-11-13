# typy-test: TYP-14
# typy-expect: rejected E0201 at 4:16
def first(value: str | bytes) -> str:
    return str(value[0])
