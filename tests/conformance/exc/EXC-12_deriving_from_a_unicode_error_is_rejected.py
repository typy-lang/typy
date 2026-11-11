# typy-test: EXC-12
# typy-expect: rejected E0417 at 3:15
class BadText(UnicodeDecodeError):
    pass
