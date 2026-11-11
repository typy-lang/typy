# typy-test: EXC-06
# typy-expect: rejected E0419 at 7:9
def f() -> int:
    try:
        return 1
    finally:
        return 2
