# typy-test: DAT-10, EXC-02
# typy-expect: accepted
names = set(["a", "b"])
names.remove("a")
print(len(names))
names.remove("z")
