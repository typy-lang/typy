"""Base16, Base32, Base64 (RFC 3548), Base85 and Ascii85 data encodings"""

# The results and messages are CPython 3.15's (ADR-0029), with the parameters Python 3.12
# has; the parameters and functions later versions add are planned for 0.13 (LIB-32).
# CPython 3.15 also writes a FutureWarning when URL-safe data, or data decoded with
# `altchars`, holds `+` or `/`; this module decodes the same bytes and writes no warning.

import binascii

__all__ = [
    "encodebytes",
    "decodebytes",
    "b64encode",
    "b64decode",
    "b32encode",
    "b32decode",
    "b32hexencode",
    "b32hexdecode",
    "b16encode",
    "b16decode",
    "b85encode",
    "b85decode",
    "a85encode",
    "a85decode",
    "standard_b64encode",
    "standard_b64decode",
    "urlsafe_b64encode",
    "urlsafe_b64decode",
]


def _bytes_from_decode_data(s: str | bytes) -> bytes:
    if isinstance(s, str):
        if not s.isascii():
            raise ValueError("string argument should contain only ASCII characters")
        return s.encode("ascii")
    return s


def _translated(data: bytes, source: bytes, target: bytes) -> bytes:
    """Returns `data` with each byte of `source` replaced by the byte of `target` at its place,
    as `bytes.translate` with `bytes.maketrans(source, target)` does."""
    table = list(range(256))
    for index in range(len(source)):
        table[source[index]] = target[index]
    return bytes([table[byte] for byte in data])


def b64encode(s: bytes, altchars: bytes | None = None) -> bytes:
    """Encode the bytes-like object s using Base64 and return a bytes object.

    Optional altchars should be a byte string of length 2 which specifies an
    alternative alphabet for the '+' and '/' characters.  This allows an
    application to e.g. generate url or filesystem safe Base64 strings.
    """
    encoded = binascii.b2a_base64(s, newline=False)
    if altchars is not None:
        if len(altchars) != 2:
            raise ValueError(f"invalid altchars: {altchars!r}")
        return _translated(encoded, b"+/", altchars)
    return encoded


def b64decode(s: str | bytes, altchars: str | bytes | None = None, validate: bool = False) -> bytes:
    """Decode the Base64 encoded bytes-like object or ASCII string s.

    Optional altchars must be a bytes-like object or ASCII string of length 2
    which specifies the alternative alphabet used instead of the '+' and '/'
    characters.

    The result is returned as a bytes object.  A binascii.Error is raised if
    s is incorrectly padded.

    If validate is false, characters that are neither in the normal base-64
    alphabet nor the alternative alphabet are discarded prior to the
    padding check.  If validate is true, these non-alphabet characters in
    the input result in a binascii.Error.
    """
    data = _bytes_from_decode_data(s)
    if altchars is not None:
        alternative = _bytes_from_decode_data(altchars)
        if len(alternative) != 2:
            raise ValueError(f"invalid altchars: {alternative!r}")
        data = _translated(data, alternative, b"+/")
    return binascii.a2b_base64(data, strict_mode=validate)


def standard_b64encode(s: bytes) -> bytes:
    """Encode bytes-like object s using the standard Base64 alphabet."""
    return b64encode(s)


def standard_b64decode(s: str | bytes) -> bytes:
    """Decode bytes encoded with the standard Base64 alphabet."""
    return b64decode(s)


def urlsafe_b64encode(s: bytes) -> bytes:
    """Encode bytes using the URL- and filesystem-safe Base64 alphabet."""
    return _translated(b64encode(s), b"+/", b"-_")


def urlsafe_b64decode(s: str | bytes) -> bytes:
    """Decode bytes using the URL- and filesystem-safe Base64 alphabet.

    Padding in the input is not required, as in CPython 3.15.
    """
    data = _translated(_bytes_from_decode_data(s), b"-_", b"+/")
    return binascii._base64_decoded(data, False, False)


_B32_ALPHABET = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
_B32_HEX_ALPHABET = b"0123456789ABCDEFGHIJKLMNOPQRSTUV"


def _b32encode(alphabet: bytes, s: bytes) -> bytes:
    leftover = len(s) % 5
    data = s + b"\0" * (5 - leftover) if leftover else s
    encoded = bytearray()
    for index in range(0, len(data), 5):
        chunk = 0
        for byte in data[index : index + 5]:
            chunk = (chunk << 8) | byte
        for shift in (35, 30, 25, 20, 15, 10, 5, 0):
            encoded.append(alphabet[(chunk >> shift) & 31])
    if leftover == 1:
        encoded[-6:] = b"======"
    elif leftover == 2:
        encoded[-4:] = b"===="
    elif leftover == 3:
        encoded[-3:] = b"==="
    elif leftover == 4:
        encoded[-1:] = b"="
    return bytes(encoded)


def _b32decode(alphabet: bytes, s: str | bytes, casefold: bool, map01: str | bytes | None) -> bytes:
    """Returns the bytes base32 `s` encodes in `alphabet`, as CPython 3.15 decodes them."""
    data = _bytes_from_decode_data(s)
    if map01 is not None:
        mapped = _bytes_from_decode_data(map01)
        if len(mapped) != 1:
            raise ValueError("maketrans arguments must have same length")
        data = _translated(data, b"01", b"O" + mapped)
    if casefold:
        data = data.upper()
    values = [32] * 256
    for index, character in enumerate(alphabet):
        values[character] = index
    decoded = bytearray()
    octa_pos = 0
    leftchar = 0
    pads = 0
    for character in data:
        if character == 61:
            pads += 1
            if octa_pos in (2, 4, 5, 7) and octa_pos + pads <= 8:
                continue
            if octa_pos in (1, 3, 6):
                break
            if octa_pos == 0 and len(decoded) == 0:
                raise binascii.Error("Leading padding not allowed")
            raise binascii.Error("Excess padding not allowed")
        value = values[character]
        if value >= 32:
            raise binascii.Error("Only base32 data is allowed")
        if pads != 0:
            if octa_pos + pads == 8:
                raise binascii.Error("Excess data after padding")
            raise binascii.Error("Discontinuous padding not allowed")
        # Each of 8 characters gives 5 bits; a byte is written once 8 bits are known.
        if octa_pos in (0, 2, 5):
            leftchar = (leftchar << 5) | value
        else:
            used = _B32_USED[octa_pos]
            decoded.append(((leftchar << used) | (value >> (5 - used))) & 255)
            leftchar = value & ((1 << (5 - used)) - 1)
        octa_pos = (octa_pos + 1) % 8
    if octa_pos in (1, 3, 6):
        raise binascii.Error(
            "Invalid base32-encoded string: number of data characters (%d) cannot be 1, 3, "
            "or 6 more than a multiple of 8" % (len(decoded) // 5 * 8 + octa_pos)
        )
    if octa_pos != 0 and octa_pos + pads < 8:
        raise binascii.Error("Incorrect padding")
    return bytes(decoded)


# The bits a character at each place of 8 adds to the byte it completes.
_B32_USED = [0, 3, 0, 1, 4, 0, 2, 5]


def b32encode(s: bytes) -> bytes:
    """Encode the bytes-like objects using base32 and return a bytes object."""
    return _b32encode(_B32_ALPHABET, s)


def b32decode(s: str | bytes, casefold: bool = False, map01: str | bytes | None = None) -> bytes:
    """Decode the base32 encoded bytes-like object or ASCII string s."""
    return _b32decode(_B32_ALPHABET, s, casefold, map01)


def b32hexencode(s: bytes) -> bytes:
    """Encode the bytes-like objects using base32hex and return a bytes object."""
    return _b32encode(_B32_HEX_ALPHABET, s)


def b32hexdecode(s: str | bytes, casefold: bool = False) -> bytes:
    """Decode the base32hex encoded bytes-like object or ASCII string s."""
    return _b32decode(_B32_HEX_ALPHABET, s, casefold, None)


def b16encode(s: bytes) -> bytes:
    """Encode the bytes-like object s using Base16 and return a bytes object."""
    return binascii.hexlify(s).upper()


def b16decode(s: str | bytes, casefold: bool = False) -> bytes:
    """Decode the Base16 encoded bytes-like object or ASCII string s.

    Optional casefold is a flag specifying whether a lowercase alphabet is
    acceptable as input.  For security purposes, the default is False.
    """
    if not casefold:
        data = _bytes_from_decode_data(s)
        for character in data:
            if 97 <= character <= 102:
                raise binascii.Error("Non-base16 digit found")
        return binascii.unhexlify(data)
    return binascii.unhexlify(s)


_A85_START = b"<~"
_A85_END = b"~>"
_A85_CHARACTERS = bytes(list(range(33, 118)))
_B85_ALPHABET = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_`{|}~"


def _base85_digits(word: int, characters: bytes, count: int) -> bytes:
    """Returns the first `count` of the 5 base85 digits of `word`, written in `characters`."""
    digits = [0] * 5
    for place in range(4, -1, -1):
        digits[place] = characters[word % 85]
        word //= 85
    return bytes(digits[:count])


def _85encode(b: bytes, characters: bytes, pad: bool, ascii85: bool, foldspaces: bool) -> bytes:
    """Returns `b` in base85 digits of `characters`; Ascii85 writes `z` for four zero
    bytes, and `y` for four spaces with `foldspaces`, as CPython 3.15 does."""
    encoded = bytearray()
    full = len(b) - len(b) % 4
    for index in range(0, full, 4):
        word = (b[index] << 24) | (b[index + 1] << 16) | (b[index + 2] << 8) | b[index + 3]
        if ascii85 and word == 0:
            encoded.extend(b"z")
        elif ascii85 and foldspaces and word == 0x20202020:
            encoded.extend(b"y")
        else:
            encoded.extend(_base85_digits(word, characters, 5))
    left = len(b) - full
    if left > 0:
        word = 0
        for place in range(4):
            word <<= 8
            if place < left:
                word |= b[full + place]
        if ascii85 and pad and word == 0:
            encoded.extend(b"z")
        else:
            encoded.extend(_base85_digits(word, characters, 5 if pad else left + 1))
    return bytes(encoded)


def _wrapped(data: bytes, width: int) -> bytes:
    """Returns `data` in lines of `width` characters, the last one shorter or equal."""
    if len(data) <= width:
        return data
    return b"\n".join([data[start : start + width] for start in range(0, len(data), width)])


def a85encode(b: bytes, *, foldspaces: bool = False, wrapcol: int = 0, pad: bool = False, adobe: bool = False) -> bytes:
    """Encode bytes-like object b using Ascii85 and return a bytes object."""
    if wrapcol < 0:
        raise ValueError("Cannot convert negative int")
    result = _85encode(b, _A85_CHARACTERS, pad, True, foldspaces)
    if adobe:
        result = _A85_START + result + _A85_END
    if wrapcol != 0 and len(result) > 0:
        result = _wrapped(result, 2 if adobe and wrapcol == 1 else wrapcol)
        # The closing `~>` stays on one line.
        if adobe and result[-2] == 10:
            result = result[:-3] + b"\n~>"
    return result


def a85decode(b: str | bytes, *, foldspaces: bool = False, adobe: bool = False, ignorechars: bytes = b" \t\n\r\v") -> bytes:
    """Decode the Ascii85 encoded bytes-like object or ASCII string b."""
    data = _bytes_from_decode_data(b)
    if adobe:
        if not data.endswith(_A85_END):
            raise binascii.Error("Ascii85 encoded byte sequences must end with b'~>'")
        data = data[:-2]
        if data.startswith(_A85_START):
            data = data[2:]
    decoded = bytearray()
    leftchar = 0
    group_pos = 0
    character = 0
    remaining = len(data)
    # After the data, each group is filled with the largest digit, as the encoder cut it.
    while remaining > 0 or group_pos != 0:
        digit = 84
        if remaining > 0:
            character = data[len(data) - remaining]
            digit = character - 33 if 33 <= character <= 117 else 85
        if digit < 85:
            if group_pos == 4 and leftchar * 85 + digit > 0xFFFFFFFF:
                raise binascii.Error("Ascii85 overflow")
            leftchar = leftchar * 85 + digit
            group_pos += 1
        elif (character == 121 and foldspaces) or character == 122:
            if group_pos != 0:
                raise binascii.Error(f"'{chr(character)}' inside Ascii85 5-tuple")
            leftchar = 0x20202020 if character == 121 else 0
            group_pos = 5
        elif character not in ignorechars:
            raise binascii.Error(f"Non-Ascii85 digit found: {chr(character)}")
        if group_pos == 5:
            _write_group(decoded, leftchar, remaining, "Ascii85")
            group_pos = 0
            leftchar = 0
        remaining -= 1
    return bytes(decoded)


def _write_group(decoded: bytearray, leftchar: int, remaining: int, encoding: str) -> None:
    """Appends the bytes of a full group to `decoded`: 4, or for the last group one fewer
    than its characters; a last group of one character is refused."""
    count = 3 + remaining if remaining < 1 else 4
    if count == 0:
        raise binascii.Error(f"Incomplete {encoding} group")
    for place in range(count):
        decoded.append((leftchar >> (24 - 8 * place)) & 255)


def b85encode(b: bytes, pad: bool = False) -> bytes:
    """Encode bytes-like object b in base85 format and return a bytes object."""
    return _85encode(b, _B85_ALPHABET, pad, False, False)


def b85decode(b: str | bytes) -> bytes:
    """Decode the base85-encoded bytes-like object or ASCII string b"""
    data = _bytes_from_decode_data(b)
    decoded = bytearray()
    leftchar = 0
    group_pos = 0
    remaining = len(data)
    while remaining > 0 or group_pos != 0:
        digit = 84
        if remaining > 0:
            digit = _B85_VALUES[data[len(data) - remaining]]
            if digit >= 85:
                raise binascii.Error("bad Base85 character at position %d" % (len(data) - remaining))
        if group_pos == 4 and leftchar * 85 + digit > 0xFFFFFFFF:
            position = (len(data) - remaining) // 5 * 5
            raise binascii.Error("Base85 overflow in hunk starting at byte %d" % position)
        leftchar = leftchar * 85 + digit
        group_pos += 1
        if group_pos == 5:
            _write_group(decoded, leftchar, remaining, "Base85")
            group_pos = 0
            leftchar = 0
        remaining -= 1
    return bytes(decoded)


def _base85_values() -> list[int]:
    values = [85] * 256
    for index, character in enumerate(_B85_ALPHABET):
        values[character] = index
    return values


_B85_VALUES = _base85_values()


_MAXLINESIZE = 76
_MAXBINSIZE = (_MAXLINESIZE // 4) * 3


def encodebytes(s: bytes) -> bytes:
    """Encode a bytestring into a bytes object containing multiple lines
    of base-64 data."""
    pieces = [binascii.b2a_base64(s[index : index + _MAXBINSIZE]) for index in range(0, len(s), _MAXBINSIZE)]
    return b"".join(pieces)


def decodebytes(s: bytes) -> bytes:
    """Decode a bytestring of base-64 data into a bytes object."""
    return binascii.a2b_base64(s)
