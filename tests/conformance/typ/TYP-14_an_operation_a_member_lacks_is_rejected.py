# typy-test: TYP-14
# typy-expect: rejected E0412 at 4:18
def encoded(value: str | bytes) -> bytes:
    return value.encode()
