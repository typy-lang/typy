# typy-test: TYP-04
# typy-expect: rejected E0402 at 7:14
def size(text: str) -> int:
    return len(text)


label: str = size("abc")
