# typy-test: CST-03
# typy-expect: accepted
for x in (10, 20, 30):
    print(x)
for i, f in enumerate((1.5, 2.5)):
    print(i, f)
for s in reversed(("a", "b")):
    print(s)
