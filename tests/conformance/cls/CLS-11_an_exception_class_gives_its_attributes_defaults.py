# typy-test: CLS-11
# typy-expect: accepted
class AppError(Exception):
    code: int = 1
    label: str = "app"
    hint: str | None = None


class NotFound(AppError):
    code: int = 404

    def __init__(self, path: str) -> None:
        super().__init__(path)
        self.path = path


for error in [AppError("x"), NotFound("/a"), AppError()]:
    try:
        raise error
    except AppError as caught:
        print(type(caught).__name__, caught.code, caught.label, caught.hint, caught.args)
try:
    raise AppError
except AppError as error:
    print("found", error.code, error.args)
e = AppError("y")
e.code += 5
e.hint = "retry"
print(e.code, e.hint, AppError.code, NotFound.code)
