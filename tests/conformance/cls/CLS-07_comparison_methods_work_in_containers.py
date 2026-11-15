# typy-test: CLS-07
# typy-expect: accepted
class Version:
    def __init__(self, major: int, minor: int) -> None:
        self.major = major
        self.minor = minor

    def __eq__(self, other: "Version") -> bool:
        return self.major == other.major and self.minor == other.minor

    def __lt__(self, other: "Version") -> bool:
        return (self.major, self.minor) < (other.major, other.minor)

    def __hash__(self) -> int:
        return self.major * 1000 + self.minor

    def __repr__(self) -> str:
        return f"Version({self.major}, {self.minor})"


versions = [Version(2, 0), Version(1, 5), Version(1, 10), Version(1, 5)]
print(Version(1, 5) in versions, Version(3, 0) in versions)
print(versions.index(Version(1, 10)), versions.count(Version(1, 5)))
print(sorted(versions), min(versions), max(versions))
unique = set(versions)
print(len(unique), Version(2, 0) in unique)
names = {Version(1, 5): "stable", Version(2, 0): "next"}
print(names[Version(1, 5)], Version(9, 9) in names)
print(versions == [Version(2, 0), Version(1, 5), Version(1, 10), Version(1, 5)])
