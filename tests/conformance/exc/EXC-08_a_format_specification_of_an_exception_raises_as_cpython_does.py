# typy-test: EXC-08
# typy-expect: accepted
def show(value: BaseException) -> None:
    try:
        print(f"{value:>5}")
    except TypeError as error:
        print(error)


show(ValueError("x"))
key: LookupError = KeyError("k")
show(key)
print(f"{key}", f"{key:}", f"{ValueError('v')!r}")
