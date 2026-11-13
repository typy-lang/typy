# typy-test: CLS-14
# typy-expect: rejected E0201 at 8:9
class AppError(Exception):
    def describe(self) -> str:
        return "app"

    def again(self) -> None:
        super().__init__("again")
