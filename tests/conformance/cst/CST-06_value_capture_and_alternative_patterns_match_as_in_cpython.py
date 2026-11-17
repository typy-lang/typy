# typy-test: CST-06
# typy-expect: accepted
def describe(code: int) -> str:
    match code:
        case 200 | 201:
            return "ok"
        case 404:
            return "missing"
        case -1:
            return "negative one"
        case other if other >= 500:
            return f"server {other}"
        case other:
            return f"code {other}"


def size(value: int | None) -> int:
    match value:
        case None:
            return 0
        case number:
            return number * 2


def flag(state: bool) -> str:
    match state:
        case True:
            return "on"
        case _:
            return "off"


for code in [200, 201, 404, -1, 503, 302]:
    print(describe(code))
print(size(None), size(21), flag(True), flag(False))
words = ["alpha", "beta", "gamma"]
match len(words):
    case 0:
        print("none")
    case 3 as count:
        print("three", count)
match words[0]:
    case "alpha" | "omega" as edge:
        print("edge", edge)
    case _:
        print("middle")
seen = 0
for word in words:
    match word:
        case "beta":
            continue
        case "gamma":
            break
    seen += 1
print(seen)
match 7:
    case 1:
        print("one")
print("after a match that matched nothing")
