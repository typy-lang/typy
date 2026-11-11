# typy-test: CST-04, STD-05
# typy-expect: accepted
def work() -> None:
    try:
        raise ValueError("first")
    finally:
        print("cleaning up")


try:
    work()
finally:
    print("top finally")
