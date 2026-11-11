# typy-test: CLS-03, CLS-14
# typy-expect: accepted
class HttpError(Exception):
    def __init__(self, status: int, reason: str) -> None:
        super().__init__(status, reason)
        self.status = status
        self.reason = reason


class RetryableError(HttpError):
    def __init__(self, status: int, reason: str, delay: float) -> None:
        super().__init__(status, reason)
        self.delay = delay
        self.attempts: list[int] = []


errors: list[HttpError] = [HttpError(404, "missing"), RetryableError(503, "busy", 1.5)]
for error in errors:
    print(error.status, error.reason, repr(error), str(error))
retry = RetryableError(429, "slow down", 0.25)
retry.attempts.append(1)
retry.status += 1
print(retry.status, retry.delay, retry.attempts, retry)
try:
    raise retry
except HttpError as caught:
    print("caught", caught.status, type(caught).__name__)
