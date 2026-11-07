# typy-test: CST-02
# typy-expect: accepted
step = 3
for i in range(10, 0, -step):
    print(i)
for i in range(0, 7, step):
    print(i)
step = 0
for i in range(0, 7, step):
    print("never", i)
