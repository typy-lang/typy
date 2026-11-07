# typy-test: CST-01
# typy-expect: accepted
for n in range(-1, 2):
    if n < 0:
        print("negative")
    elif n == 0:
        print("zero")
    else:
        print("positive")
