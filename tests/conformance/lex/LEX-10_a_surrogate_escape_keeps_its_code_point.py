# typy-test: LEX-10
# typy-expect: accepted
s = "a\ud800b"
print(len(s), repr(s), s[1] == "\ud800", ord(s[1]), ord("\U0000dfff"))
print("\udcff".encode("utf-8", "surrogateescape"))
joined = "x" "\udc80" + s[2:]
print(repr(joined), len(joined), joined.upper().isupper())
text = f"[{len(s)}\ud83d]"
print(repr(text), repr(f"{s!r:>12}"), "\ud800" in s, s.find("\ud800"))
pairs = {"\ud800": 1, "\udc00": 2}
print(pairs["\ud800"], sorted(pairs.values()), "\ud800" < "\ue000", chr(0xd800) == "\ud800")
try:
    print(s)
except UnicodeEncodeError as error:
    print("UnicodeEncodeError:", error)
