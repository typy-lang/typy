# typy-test: MOD-08, MOD-06
# typy-expect: accepted
# typy-python: 3.15
import tries
lazy from flaky import READY


def show() -> None:
    print(READY)


for attempt in range(3):
    try:
        show()
    except ValueError as error:
        print("attempt", attempt, error, tries.count)
print(tries.count)
