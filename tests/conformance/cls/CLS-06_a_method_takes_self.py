# typy-test: CLS-06
# typy-expect: rejected E0404 at 4:9
class Tools:
    def helper() -> int:
        return 1
