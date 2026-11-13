# typy-test: EXC-04, STD-05
# typy-expect: accepted
def load(name: str) -> str:
    raise RuntimeError(f"cannot load {name}") from KeyError(name)


print(load("settings"))
