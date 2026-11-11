tries = 0


class AppError(Exception):
    pass


def fail(code: int) -> None:
    raise AppError("failed", code)
