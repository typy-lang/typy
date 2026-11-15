# typy-test: EXC-15
# typy-expect: accepted
for text in ["plain", "caf\xe9", ""]:
    try:
        if not text:
            raise ValueError("empty", 0)
        print(text.encode("ascii"))
    except ValueError as error:
        print(type(error).__name__, len(error.args), error.args[0])
        if len(error.args) == 5:
            print(error.args[2], error.args[3], error.args[4])
try:
    b"\xff".decode()
except UnicodeDecodeError as error:
    print(error.args[1], error.args[4])
