# typy-test: CLS-07
# typy-expect: accepted
class Version:
    def __init__(self, major: int, minor: int) -> None:
        self.major = major
        self.minor = minor

    def __eq__(self, other: "Version") -> bool:
        return self.major == other.major and self.minor == other.minor

    def __lt__(self, other: "Version") -> bool:
        if self.major != other.major:
            return self.major < other.major
        return self.minor < other.minor

    def __hash__(self) -> int:
        return self.major * 1000 + self.minor


class Release(Version):
    def __eq__(self, other: "Version") -> bool:
        return self.major == other.major


a = Version(1, 2)
b = Version(1, 3)
r = Release(1, 9)
print(a == b, a != b, a < b, a > b, b > a, a == Version(1, 2), a != Version(1, 2))
print(r == a, a == r, r != b, r < b)


hidden: Version = Release(1, 0)
print(a == hidden, hidden == a, Version(1, 0) == hidden)
