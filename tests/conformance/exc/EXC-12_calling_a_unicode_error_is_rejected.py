# typy-test: EXC-12
# typy-expect: rejected E0417 at 3:7
raise UnicodeDecodeError("utf-8", b"\xff", 0, 1, "invalid start byte")
