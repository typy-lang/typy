# typy-test: EXC-12
# typy-expect: accepted
try:
    b"ok \xff".decode()
except UnicodeDecodeError as error:
    print("decode:", error)
try:
    "\u00e9".encode("ascii")
except UnicodeError as error:
    print("encode:", error)
