name = "caf" + chr(233)
total = 0
for index in range(len(name)):
    total += ord(name[index])
label: str = name + "!" if total > 400 else "?"
print(label, repr(label), len(label), end="\n")
print(ord(name[-1]) == 233 and total % 2 == 0)
