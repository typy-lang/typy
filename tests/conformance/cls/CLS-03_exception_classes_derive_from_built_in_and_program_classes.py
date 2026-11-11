# typy-test: CLS-03, CLS-13, CLS-14, EXC-01, STD-05
# typy-expect: accepted
class AppError(Exception):
    pass


class ConfigError(AppError):
    def __init__(self, key: str, line: int) -> None:
        super().__init__(f"bad key {key}")
        self.key = key
        self.line = line

    def where(self) -> str:
        return f"{self.key} at line {self.line}"


class MissingKey(ConfigError):
    pass


class Quiet(ValueError):
    def __str__(self) -> str:
        return "quiet!"


def load(key: str) -> int:
    if key == "port":
        raise MissingKey(key, 7)
    if key == "host":
        raise Quiet("host", 1)
    if key == "app":
        raise AppError
    return 1


for key in ["port", "host", "app", "user"]:
    try:
        print(key, load(key))
    except ConfigError as error:
        print("config:", error, repr(error), error.where(), error.key, type(error).__name__)
    except ValueError as error:
        print("value:", error, repr(error), type(error).__name__)
    except AppError as error:
        print("app:", repr(error), str(error) == "")
e = MissingKey("k", 2)
print(e.line)
raise MissingKey("timeout", 3)
