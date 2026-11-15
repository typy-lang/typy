# typy-test: EXC-14
# typy-expect: accepted
def show(error: OSError) -> None:
    print(type(error).__name__, error, repr(error), error.args)
    print(error.errno, error.strerror, error.filename, error.filename2)


try:
    raise OSError(2, "No such file or directory")
except FileNotFoundError as error:
    show(error)
try:
    raise OSError(13, "Permission denied", "secret.txt")
except PermissionError as error:
    show(error)
try:
    raise OSError(2, "missing", "a.txt", None, "b.txt")
except OSError as error:
    show(error)
try:
    raise OSError(999, "odd", None)
except OSError as error:
    show(error)
try:
    raise FileNotFoundError(13, "kept class")
except OSError as error:
    show(error)
try:
    raise OSError("one message")
except OSError as error:
    show(error)
try:
    open("/no/such/file.txt")
except OSError as error:
    show(error)
try:
    raise OSError(2, "x")
except OSError as error:
    number = error.errno
    if number is not None and number == 2 and isinstance(error, FileNotFoundError):
        print("matched", error.errno is not None)
