# typy-test: LIB-04, EXC-02
# typy-expect: accepted
try:
    open("missing.txt")
except FileNotFoundError as error:
    print(type(error).__name__, error, repr(error))
try:
    open(".", "w")
except IsADirectoryError as error:
    print(error)
try:
    open(".")
except OSError as error:
    print(type(error).__name__, error)
open("there.txt", "w").close()
try:
    open("there.txt", "x")
except FileExistsError as error:
    print(error)
try:
    open("there.txt/inside.txt")
except NotADirectoryError as error:
    print(error)
try:
    open("missing/inside.txt", "w")
except IOError as error:
    print("IOError", type(error).__name__)
try:
    open("missing.txt")
except EnvironmentError:
    print("EnvironmentError catches it")
try:
    open("a\x00b")
except ValueError as error:
    print(error)
try:
    raise OSError("one message")
except OSError as error:
    print(repr(error), error)
try:
    raise ConnectionResetError()
except ConnectionError as error:
    print(repr(error), "ConnectionError catches it")
