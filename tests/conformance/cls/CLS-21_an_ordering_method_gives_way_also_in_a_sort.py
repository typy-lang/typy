# typy-test: CLS-21
# typy-expect: accepted
class Version:
    def __init__(self, major: int) -> None:
        self.major = major

    def __lt__(self, other: "Version") -> bool:
        if other.major < 0:
            return NotImplemented
        return self.major < other.major

    def __gt__(self, other: "Version") -> bool:
        if other.major < 0:
            return NotImplemented
        return self.major > other.major


print(Version(1) < Version(2), Version(3) > Version(2))
print([v.major for v in sorted([Version(3), Version(1), Version(2)])])
print(min([Version(3), Version(1)]).major, max([Version(3), Version(1)]).major)
for pair in [(Version(1), Version(-1)), (Version(-1), Version(1))]:
    try:
        print(pair[0] < pair[1])
    except TypeError as error:
        print("TypeError:", error)
try:
    print([v.major for v in sorted([Version(2), Version(-1), Version(1)])])
except TypeError as error:
    print("TypeError:", error)
