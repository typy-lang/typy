# typy-test: CLS-03, CLS-01
# typy-expect: rejected E0309 at 9:5
class HttpError(Exception):
    def __init__(self, status: int) -> None:
        self.status = status


class Missing(HttpError):
    def __init__(self, path: str) -> None:
        self.path = path
