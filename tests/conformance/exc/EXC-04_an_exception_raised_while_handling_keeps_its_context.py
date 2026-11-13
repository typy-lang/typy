# typy-test: EXC-04, STD-05
# typy-expect: accepted
def convert(text: str) -> int:
    try:
        return int(text)
    except ValueError:
        raise RuntimeError("cannot convert " + text)


print(convert("5"))
print(convert("five"))
