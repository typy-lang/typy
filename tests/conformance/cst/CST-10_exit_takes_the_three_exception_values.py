# typy-test: CST-10
# typy-expect: rejected E0404 at 7:9
class Wrong:
    def __enter__(self) -> None:
        return None

    def __exit__(self, kind: int, error, trace) -> None:
        return None
