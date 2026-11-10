# typy-test: CST-04
# typy-expect: accepted
def parse(text: str) -> int:
    try:
        value = int(text)
    except (ValueError, TypeError) as error:
        print("bad input:", error)
        return 0
    else:
        print("parsed", value)
    return value


print(parse("12"), parse("twelve"))
