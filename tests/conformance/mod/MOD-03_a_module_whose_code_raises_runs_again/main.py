# typy-test: MOD-03, EXC-02
# typy-expect: accepted
import errors


def late() -> str:
    import fragile
    return fragile.VALUE


for attempt in range(2):
    try:
        import fragile
    except ValueError as error:
        print("attempt", attempt, error)
print(late(), errors.tries)
try:
    errors.fail(3)
except errors.AppError as error:
    print("caught", error, type(error).__name__)
