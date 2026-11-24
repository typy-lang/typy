# typy-test: LIB-27
# typy-expect: accepted
# base64 and binascii encode and decode as CPython's modules do, raising binascii.Error for
# input that is padded wrongly or holds other digits.
import base64
import binascii

data = b"TyPy compiles typed Python\x00\xff"
print(base64.b64encode(data), base64.b64decode(base64.b64encode(data)) == data)
print(base64.urlsafe_b64encode(data), base64.urlsafe_b64decode(base64.urlsafe_b64encode(data)) == data)
print(base64.b32encode(data), base64.b32decode(base64.b32encode(data)) == data)
print(base64.b32hexencode(data), base64.b32hexdecode(base64.b32hexencode(data)) == data)
print(base64.b16encode(data), base64.b16decode(base64.b16encode(data)) == data)
print(base64.b85encode(data), base64.b85decode(base64.b85encode(data)) == data)
print(base64.a85encode(data), base64.a85decode(base64.a85encode(data)) == data)
print(base64.b64encode(b"\xfb\xff", altchars=b"-_"), base64.b64decode("+/8="), base64.b64decode(b"aGk="))
print(base64.standard_b64encode(b"ok"), base64.standard_b64decode(b"b2s="), base64.encodebytes(b"x" * 60))
print(base64.decodebytes(b"aGVsbG8=\n"), base64.b32decode("me======", casefold=True), base64.b16decode("ff", casefold=True))
print(base64.a85encode(b"\0\0\0\0    ", foldspaces=True, adobe=True), base64.b85encode(b"ab", pad=True))
print(binascii.hexlify(b"\xb9\x01\xef", "-"), binascii.unhexlify("b901ef"), binascii.crc32(b"hello"))
print(binascii.b2a_base64(b"hi"), binascii.a2b_base64(b"aGk="), binascii.b2a_hex(b"\x01\x02", b":", 1))
for bad in (b"YQ", b"Y==="):
    try:
        base64.b64decode(bad)
    except binascii.Error as error:
        print("binascii.Error", error)
try:
    base64.b32decode(b"AAAAAAA")
except binascii.Error as error:
    print("binascii.Error", error)
