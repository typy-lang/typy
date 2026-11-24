"""Conversion between binary data and ASCII"""

# The results and messages are CPython 3.15's (ADR-0029), with the parameters Python 3.12
# has. `a2b_uu`, `b2a_uu`, `a2b_qp`, `b2a_qp` and `crc_hqx` are planned for 0.13 (LIB-31).


class Error(ValueError):
    pass


class Incomplete(Exception):
    pass


_BASE64 = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
_HEX = b"0123456789abcdef"


def _ascii(data: str | bytes) -> bytes:
    """Returns the bytes of `data`, which a `str` gives only of ASCII characters."""
    if isinstance(data, str):
        if not data.isascii():
            raise ValueError("string argument should contain only ASCII characters")
        return data.encode("ascii")
    return data


def _base64_values() -> list[int]:
    values = [64] * 256
    for index, character in enumerate(_BASE64):
        values[character] = index
    return values


_BASE64_VALUES = _base64_values()


def b2a_base64(data: bytes, /, *, newline: bool = True) -> bytes:
    """Base64-code line of data."""
    encoded = bytearray()
    full = len(data) - len(data) % 3
    for index in range(0, full, 3):
        chunk = (data[index] << 16) | (data[index + 1] << 8) | data[index + 2]
        encoded.append(_BASE64[chunk >> 18])
        encoded.append(_BASE64[(chunk >> 12) & 63])
        encoded.append(_BASE64[(chunk >> 6) & 63])
        encoded.append(_BASE64[chunk & 63])
    left = len(data) - full
    if left == 1:
        chunk = data[full] << 16
        encoded.append(_BASE64[chunk >> 18])
        encoded.append(_BASE64[(chunk >> 12) & 63])
        encoded.extend(b"==")
    elif left == 2:
        chunk = (data[full] << 16) | (data[full + 1] << 8)
        encoded.append(_BASE64[chunk >> 18])
        encoded.append(_BASE64[(chunk >> 12) & 63])
        encoded.append(_BASE64[(chunk >> 6) & 63])
        encoded.extend(b"=")
    if newline:
        encoded.extend(b"\n")
    return bytes(encoded)


def a2b_base64(data: str | bytes, /, *, strict_mode: bool = False) -> bytes:
    """Decode a line of base64 data.

      strict_mode
        When set to true, bytes that are not part of the base64 standard are
        not allowed.  The same applies to excess data after padding (= / ==).
    """
    return _base64_decoded(_ascii(data), strict_mode, True)


def _base64_decoded(text: bytes, strict_mode: bool, padded: bool) -> bytes:
    """Returns the bytes base64 `text` encodes, as CPython 3.15 decodes it. Without
    `padded`, padding is optional and `=` is a character outside the alphabet."""
    decoded = bytearray()
    quad_pos = 0
    leftchar = 0
    pads = 0
    for character in text:
        if padded and character == 61:
            pads += 1
            # RFC 4648, section 3.3: padding inside the data, and excess padding, may be
            # skipped.
            if (quad_pos >= 2 and quad_pos + pads <= 4) or not strict_mode:
                continue
            if quad_pos == 1:
                break
            if quad_pos == 0 and len(decoded) == 0:
                raise Error("Leading padding not allowed")
            raise Error("Excess padding not allowed")
        value = _BASE64_VALUES[character]
        if value >= 64:
            if strict_mode:
                raise Error("Padding not allowed" if character == 61 else "Only base64 data is allowed")
            continue
        if pads != 0 and strict_mode:
            if quad_pos + pads == 4:
                raise Error("Excess data after padding")
            raise Error("Discontinuous padding not allowed")
        pads = 0
        if quad_pos == 0:
            quad_pos = 1
            leftchar = value
        elif quad_pos == 1:
            quad_pos = 2
            decoded.append(((leftchar << 2) | (value >> 4)) & 255)
            leftchar = value & 15
        elif quad_pos == 2:
            quad_pos = 3
            decoded.append(((leftchar << 4) | (value >> 2)) & 255)
            leftchar = value & 3
        else:
            quad_pos = 0
            decoded.append(((leftchar << 6) | value) & 255)
            leftchar = 0
    if quad_pos == 1:
        raise Error(
            "Invalid base64-encoded string: number of data characters (%d) cannot be 1 more "
            "than a multiple of 4" % (len(decoded) // 3 * 4 + 1)
        )
    if padded and quad_pos != 0 and quad_pos + pads < 4:
        raise Error("Incorrect padding")
    return bytes(decoded)


def hexlify(data: bytes, sep: str | bytes | None = None, bytes_per_sep: int = 1) -> bytes:
    """Hexadecimal representation of binary data."""
    digits = bytearray()
    for byte in data:
        digits.append(_HEX[byte >> 4])
        digits.append(_HEX[byte & 15])
    if sep is None or bytes_per_sep == 0 or len(data) == 0:
        return bytes(digits)
    separator = _ascii(sep)
    if len(separator) != 1:
        raise ValueError("sep must be length 1.")
    group = abs(bytes_per_sep) * 2
    pieces: list[bytes] = []
    if bytes_per_sep > 0:
        first = len(digits) % group or group
        pieces.append(bytes(digits[:first]))
        for start in range(first, len(digits), group):
            pieces.append(bytes(digits[start : start + group]))
    else:
        for start in range(0, len(digits), group):
            pieces.append(bytes(digits[start : start + group]))
    return separator.join(pieces)


def b2a_hex(data: bytes, sep: str | bytes | None = None, bytes_per_sep: int = 1) -> bytes:
    """Hexadecimal representation of binary data."""
    return hexlify(data, sep, bytes_per_sep)


def _hex_value(character: int) -> int:
    """Returns the value of a hexadecimal digit, or 16 for any other character."""
    if 48 <= character <= 57:
        return character - 48
    if 97 <= character <= 102:
        return character - 87
    if 65 <= character <= 70:
        return character - 55
    return 16


def unhexlify(hexstr: str | bytes, /) -> bytes:
    """Binary data of hexadecimal representation."""
    decoded = bytearray()
    high = -1
    for character in _ascii(hexstr):
        value = _hex_value(character)
        if value >= 16:
            raise Error("Non-hexadecimal digit found")
        if high < 0:
            high = value
        else:
            decoded.append((high << 4) | value)
            high = -1
    if high >= 0:
        raise Error("Odd number of hexadecimal digits")
    return bytes(decoded)


def a2b_hex(hexstr: str | bytes, /) -> bytes:
    """Binary data of hexadecimal representation."""
    return unhexlify(hexstr)


def _crc_table() -> list[int]:
    table: list[int] = []
    for index in range(256):
        value = index
        for _bit in range(8):
            if value & 1:
                value = (value >> 1) ^ 0xEDB88320
            else:
                value >>= 1
        table.append(value)
    return table


_CRC_TABLE = _crc_table()


def crc32(data: bytes, crc: int = 0, /) -> int:
    """Compute CRC-32 incrementally."""
    value = ~crc & 0xFFFFFFFF
    for byte in data:
        value = _CRC_TABLE[(value ^ byte) & 255] ^ (value >> 8)
    return ~value & 0xFFFFFFFF
