# typy-test: LIB-14
# typy-expect: accepted
with open("e.txt", "w") as out:
    out.write("line\n")
base = open("e.txt")
number = base.fileno()
shared = open(number, closefd=False)
print(shared.name == number, shared.mode, repr(shared.read()))
shared.close()
print(repr(base.read()), base.seek(0))
binary = open(number, "rb", closefd=False)
print(binary.name == number, repr(binary.read()))
binary.close()
raw = open(number, "rb", buffering=0, closefd=False)
print(raw.read(), repr(raw).endswith("mode='rb' closefd=False>"))
raw.close()
print(repr(raw))
owned = open(number)
owned.close()
try:
    base.read()
except OSError as error:
    print(type(error).__name__, error)
try:
    base.close()
except OSError as error:
    print("close", error)
for descriptor in [-1, 99, 1 << 40]:
    try:
        open(descriptor)
    except ValueError as error:
        print("ValueError", error)
    except OSError as error:
        print("OSError", error)
    except TypeError as error:
        print("TypeError", error)
keep = False
try:
    open("e.txt", closefd=keep)
except ValueError as error:
    print(error)
with open("e.txt", closefd=True) as again:
    print(repr(again.read()))
