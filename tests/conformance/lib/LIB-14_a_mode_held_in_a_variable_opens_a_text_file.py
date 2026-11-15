# typy-test: LIB-14
# typy-expect: accepted
def open_with(mode: str) -> None:
    try:
        file = open("m.txt", mode)
        print(mode, repr(file), file.mode, file.readable(), file.writable())
        file.close()
    except ValueError as error:
        print(mode, "ValueError", error)
    except OSError as error:
        print(mode, type(error).__name__, error)


for mode in ["w", "r", "a+", "rt", "r+", "x", "z", "rw", "t", "+", "rr", "bt", ""]:
    open_with(mode)
mode = "w"
with open("m.txt", mode, encoding="latin-1") as out:
    out.write("caf\u00e9\n")
with open("m.txt", "r" + "", encoding="latin-1") as back:
    print(repr(back.read()))
