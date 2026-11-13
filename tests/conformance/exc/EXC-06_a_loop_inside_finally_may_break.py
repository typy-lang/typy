# typy-test: EXC-06
# typy-expect: accepted
try:
    print("body")
finally:
    for i in range(5):
        if i == 2:
            break
        print("finally", i)
