# typy-test: EXC-19
# typy-expect: accepted
for number in range(1, 134):
    error = OSError(number, "text")
    print(number, type(error).__name__)
try:
    open("/no/such/directory/file")
except OSError as error:
    print(type(error).__name__, error.errno, error.strerror)
