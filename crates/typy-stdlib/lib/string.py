"""A collection of string constants.

Public module variables:

whitespace -- a string containing all ASCII whitespace
ascii_lowercase -- a string containing all ASCII lowercase letters
ascii_uppercase -- a string containing all ASCII uppercase letters
ascii_letters -- a string containing all ASCII letters
digits -- a string containing all ASCII decimal digits
hexdigits -- a string containing all ASCII hexadecimal digits
octdigits -- a string containing all ASCII octal digits
punctuation -- a string containing all ASCII punctuation characters
printable -- a string containing all ASCII characters considered printable

"""

from collections.abc import Mapping
from typing import ClassVar

__all__ = [
    "ascii_letters",
    "ascii_lowercase",
    "ascii_uppercase",
    "capwords",
    "digits",
    "hexdigits",
    "octdigits",
    "printable",
    "punctuation",
    "whitespace",
    "Template",
]

whitespace = " \t\n\r\v\f"
ascii_lowercase = "abcdefghijklmnopqrstuvwxyz"
ascii_uppercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
ascii_letters = ascii_lowercase + ascii_uppercase
digits = "0123456789"
hexdigits = digits + "abcdef" + "ABCDEF"
octdigits = "01234567"
punctuation = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"
printable = digits + ascii_letters + punctuation + whitespace


def capwords(s: str, sep: str | None = None) -> str:
    """capwords(s [,sep]) -> string

    Split the argument into words using split, capitalize each
    word using capitalize, and join the capitalized words using
    join.  If the optional second argument sep is absent or None,
    runs of whitespace characters are replaced by a single space
    and leading and trailing whitespace are removed, otherwise
    sep is used to split and join the words.

    """
    words = s.split() if sep is None else s.split(sep)
    joiner = sep if sep else " "
    return joiner.join([word.capitalize() for word in words])


def _identifier_end(text: str, start: int) -> int:
    """Returns where the ASCII identifier at `start` ends, or `start` when none starts there."""
    if start >= len(text) or not (text[start] == "_" or text[start] in ascii_letters):
        return start
    end = start + 1
    while end < len(text) and (text[end] == "_" or text[end] in ascii_letters or text[end] in digits):
        end += 1
    return end


class Template:
    """A string class for supporting $-substitutions."""

    delimiter: ClassVar[str] = "$"

    def __init__(self, template: str) -> None:
        self.template = template

    def _placeholders(self) -> list[tuple[int, int, str, str]]:
        """Returns each placeholder of the template in order: where it starts and ends, its
        kind, `escaped`, `named`, `braced` or `invalid`, and the identifier it names."""
        found: list[tuple[int, int, str, str]] = []
        text = self.template
        delimiter = self.delimiter
        position = text.find(delimiter)
        while position >= 0:
            after = position + len(delimiter)
            if text.startswith(delimiter, after):
                found.append((position, after + len(delimiter), "escaped", ""))
                position = text.find(delimiter, after + len(delimiter))
                continue
            end = _identifier_end(text, after)
            if end > after:
                found.append((position, end, "named", text[after:end]))
                position = text.find(delimiter, end)
                continue
            if text.startswith("{", after):
                end = _identifier_end(text, after + 1)
                if end > after + 1 and text.startswith("}", end):
                    found.append((position, end + 1, "braced", text[after + 1 : end]))
                    position = text.find(delimiter, end + 1)
                    continue
            found.append((position, after, "invalid", ""))
            position = text.find(delimiter, after)
        return found

    def _invalid(self, index: int) -> None:
        lines = self.template[:index].splitlines(keepends=True)
        if not lines:
            colno = 1
            lineno = 1
        else:
            colno = index - len("".join(lines[:-1]))
            lineno = len(lines)
        raise ValueError("Invalid placeholder in string: line %d, col %d" % (lineno, colno))

    def substitute[V](self, mapping: Mapping[str, V] | None = None, /, **kws: V) -> str:
        """Returns the template with each placeholder replaced by its value, raising
        KeyError for a name with no value and ValueError for a placeholder of no form."""
        pieces: list[str] = []
        done = 0
        for start, end, kind, name in self._placeholders():
            pieces.append(self.template[done:start])
            if kind == "escaped":
                pieces.append(self.delimiter)
            elif kind == "invalid":
                self._invalid(end)
            elif name in kws:
                pieces.append(str(kws[name]))
            elif mapping is not None:
                pieces.append(str(mapping[name]))
            else:
                raise KeyError(name)
            done = end
        pieces.append(self.template[done:])
        return "".join(pieces)

    def safe_substitute[V](self, mapping: Mapping[str, V] | None = None, /, **kws: V) -> str:
        """Returns the template with each placeholder that has a value replaced by it, and
        every other placeholder left as it is written."""
        pieces: list[str] = []
        done = 0
        for start, end, kind, name in self._placeholders():
            pieces.append(self.template[done:start])
            if kind == "escaped":
                pieces.append(self.delimiter)
            elif kind == "invalid":
                pieces.append(self.template[start:end])
            elif name in kws:
                pieces.append(str(kws[name]))
            elif mapping is not None and name in mapping:
                pieces.append(str(mapping[name]))
            else:
                pieces.append(self.template[start:end])
            done = end
        pieces.append(self.template[done:])
        return "".join(pieces)

    def is_valid(self) -> bool:
        """Returns whether the template has no placeholder of no form."""
        return all(kind != "invalid" for _start, _end, kind, _name in self._placeholders())

    def get_identifiers(self) -> list[str]:
        """Returns the identifiers the template's placeholders name, each once, in order."""
        found: list[str] = []
        for _start, _end, kind, name in self._placeholders():
            if kind in ("named", "braced") and name not in found:
                found.append(name)
        return found
