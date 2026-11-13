# typy-test: CLS-13
# typy-expect: rejected E0201 at 9:9
class AppError(Exception):
    def code(self) -> int:
        return 1


class OtherError(AppError):
    def code(self) -> int:
        return 2
