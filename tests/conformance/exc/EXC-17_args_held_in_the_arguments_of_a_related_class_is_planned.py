# typy-test: EXC-17
# typy-expect: rejected E0201 at 7:26
def parse(text: str) -> int:
    try:
        return int(text)
    except ValueError as error:
        raise ValueError(error.args)


print(parse("1"))
