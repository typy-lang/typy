# typy-test: CLS-22
# typy-expect: rejected E0407 at 11:33
class Version:
    def __init__(self, major: int) -> None:
        self.major = major

    def __lt__(self, other: "Version") -> bool:
        return self.major < other.major


print(Version(1) < Version(2) < 3)
