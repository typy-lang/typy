# typy-test: CST-02
# typy-expect: accepted
n = 0
while n < 3:
    n += 1
else:
    print("while done", n)
for i in range(10, 0, -3):
    print(i)
else:
    print("for done")
for i in range(5):
    if i == 2:
        break
else:
    print("never")
