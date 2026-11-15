# typy-test: CLS-11
# typy-expect: rejected E0414 at 7:9
class Config:
    retries = 3

    def more(self) -> None:
        self.retries += 1
