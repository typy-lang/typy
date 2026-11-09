# typy-test: LIB-03
# typy-expect: accepted
text = "h\u00e9llo \u20ac\U0001f600"
data = text.encode()
print(data, text.encode("utf-8"), text.encode(encoding="UTF8", errors="replace"), len(data))
print(data.decode(), data.decode("utf-8") == text, b"caf\xe9".decode("latin-1"), "\u00e9".encode("latin1"))
print(text.encode("ascii", "ignore"), text.encode("ascii", "backslashreplace"), text.encode("ascii", errors="xmlcharrefreplace"))
print(b"a\xffb\xe2\x82".decode("utf-8", "replace"), b"a\xffb".decode(errors="backslashreplace"), b"\xff".decode("utf-8", "surrogateescape").encode("utf-8", "surrogateescape"))
print(bytes("abc", "ascii"), bytes("\u00e9", encoding="utf-8"), bytes("\u00e9", "ascii", "replace"))
print(b"\xe2\x82".decode())
