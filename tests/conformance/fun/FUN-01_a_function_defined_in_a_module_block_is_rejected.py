# typy-test: FUN-01
# typy-expect: rejected E0202 at 5:5
ready = True
if ready:
    def answer() -> int:
        return 1
