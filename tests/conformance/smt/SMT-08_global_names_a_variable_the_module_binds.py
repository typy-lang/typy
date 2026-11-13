# typy-test: SMT-08
# typy-expect: rejected E0423 at 4:12
def reset() -> None:
    global total
    total = 0
