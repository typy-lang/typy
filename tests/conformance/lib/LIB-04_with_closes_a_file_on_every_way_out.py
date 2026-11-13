# typy-test: LIB-04, CST-05
# typy-expect: accepted
def first_line(path: str) -> str:
    with open(path) as file:
        return file.readline()


with open("data.txt", "w") as out:
    out.write("one\ntwo\n")
print(repr(first_line("data.txt")))
try:
    with open("data.txt") as source:
        raise KeyError(source.readline())
except KeyError as error:
    print("KeyError", error)
for attempt in range(3):
    with open("data.txt", "a") as out:
        out.write(str(attempt))
        if attempt == 1:
            break
print(repr(open("data.txt").read()))
