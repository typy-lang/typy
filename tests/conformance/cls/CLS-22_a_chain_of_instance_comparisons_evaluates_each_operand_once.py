# typy-test: CLS-22
# typy-expect: accepted
class Version:
    def __init__(self, major: int) -> None:
        self.major = major

    def __lt__(self, other: "Version") -> bool:
        print("lt", self.major, other.major)
        return self.major < other.major

    def __eq__(self, other: "Version") -> bool:
        print("eq", self.major, other.major)
        return self.major == other.major


def make(major: int) -> Version:
    print("make", major)
    return Version(major)


print(make(1) < make(2) < make(3))
print(make(1) < make(3) < make(2))
print(make(2) < make(1) < make(3))
print(make(1) == make(1) < make(2) != make(5))
print(Version(1) < Version(2) == Version(2))
low = Version(1)
items = [Version(2), Version(3)]
print(low < items[0] < items[1], all([low < item < Version(9) for item in items]))


def between(value: Version, start: Version, end: Version) -> bool:
    return start < value < end


print(between(Version(5), Version(1), Version(9)), between(Version(0), Version(1), Version(9)))
