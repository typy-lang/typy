"""Text wrapping and filling."""

# The results and messages are CPython 3.15's (ADR-0029). CPython splits text into chunks
# with the regular expression `wordsep_re`; `_chunk_end` finds the same chunks by hand,
# alternative by alternative, and the regular-expression attributes of TextWrapper are
# left out.

from collections.abc import Callable

__all__ = ["TextWrapper", "wrap", "fill", "dedent", "indent", "shorten"]

# Hardcode the recognized whitespace characters to the US-ASCII whitespace characters.
_whitespace = "\t\n\x0b\x0c\r "


class TextWrapper:
    """
    Object for wrapping/filling text.  The public interface consists of
    the wrap() and fill() methods; the other methods are just there for
    subclasses to override in order to tweak the default behaviour.
    If you want to completely replace the main wrapping algorithm,
    you'll probably have to override _wrap_chunks().
    """

    def __init__(
        self,
        width: int = 70,
        initial_indent: str = "",
        subsequent_indent: str = "",
        expand_tabs: bool = True,
        replace_whitespace: bool = True,
        fix_sentence_endings: bool = False,
        break_long_words: bool = True,
        drop_whitespace: bool = True,
        break_on_hyphens: bool = True,
        tabsize: int = 8,
        *,
        max_lines: int | None = None,
        placeholder: str = " [...]",
    ) -> None:
        self.width = width
        self.initial_indent = initial_indent
        self.subsequent_indent = subsequent_indent
        self.expand_tabs = expand_tabs
        self.replace_whitespace = replace_whitespace
        self.fix_sentence_endings = fix_sentence_endings
        self.break_long_words = break_long_words
        self.drop_whitespace = drop_whitespace
        self.break_on_hyphens = break_on_hyphens
        self.tabsize = tabsize
        self.max_lines = max_lines
        self.placeholder = placeholder

    def _munge_whitespace(self, text: str) -> str:
        """Munge whitespace in text: expand tabs and convert all other
        whitespace characters to spaces."""
        if self.expand_tabs:
            text = text.expandtabs(self.tabsize)
        if self.replace_whitespace:
            for character in _whitespace:
                text = text.replace(character, " ")
        return text

    def _split(self, text: str) -> list[str]:
        """Split the text to wrap into indivisible chunks."""
        chunks: list[str] = []
        start = 0
        while start < len(text):
            if self.break_on_hyphens:
                end = _chunk_end(text, start)
            else:
                end = _run_end(text, start, text[start] in _whitespace)
            chunks.append(text[start:end])
            start = end
        return chunks

    def _fix_sentence_endings(self, chunks: list[str]) -> None:
        """Correct for sentence endings buried in 'chunks'."""
        i = 0
        while i < len(chunks) - 1:
            if chunks[i + 1] == " " and _ends_sentence(chunks[i]):
                chunks[i + 1] = "  "
                i += 2
            else:
                i += 1

    def _handle_long_word(self, reversed_chunks: list[str], cur_line: list[str], cur_len: int, width: int) -> None:
        """Handle a chunk of text (most likely a word, not whitespace) that
        is too long to fit in any line."""
        # Figure out when indent is larger than the specified width, and make
        # sure at least one character is stripped off on every pass
        if width < 1:
            space_left = 1
        else:
            space_left = width - cur_len

        # If we're allowed to break long words, then do so: put as much
        # of the next chunk onto the current line as will fit.
        if self.break_long_words and space_left > 0:
            end = space_left
            chunk = reversed_chunks[-1]
            if self.break_on_hyphens and len(chunk) > space_left:
                # break after last hyphen, but only if there are
                # non-hyphens before it
                hyphen = chunk.rfind("-", 0, space_left)
                if hyphen > 0 and any(c != "-" for c in chunk[:hyphen]):
                    end = hyphen + 1
            cur_line.append(chunk[:end])
            reversed_chunks[-1] = chunk[end:]

        # Otherwise, we have to preserve the long word intact.  Only add
        # it to the current line if there's nothing already there --
        # that minimizes how much we violate the width constraint.
        elif len(cur_line) == 0:
            cur_line.append(reversed_chunks.pop())

    def _wrap_chunks(self, chunks: list[str]) -> list[str]:
        """Wrap a sequence of text chunks and return a list of lines of
        length 'self.width' or less."""
        lines: list[str] = []
        if self.width <= 0:
            raise ValueError("invalid width %r (must be > 0)" % self.width)
        max_lines = self.max_lines
        if max_lines is not None:
            if max_lines > 1:
                indent = self.subsequent_indent
            else:
                indent = self.initial_indent
            if len(indent) + len(self.placeholder.lstrip()) > self.width:
                raise ValueError("placeholder too large for max width")

        # Arrange in reverse order so items can be efficiently popped
        # from a stack of chucks.
        chunks.reverse()

        while len(chunks) > 0:
            # Start the list of chunks that will make up the current line.
            # cur_len is just the length of all the chunks in cur_line.
            cur_line: list[str] = []
            cur_len = 0

            # Figure out which static string will prefix this line.
            if len(lines) > 0:
                indent = self.subsequent_indent
            else:
                indent = self.initial_indent

            # Maximum width for this line.
            width = self.width - len(indent)

            # First chunk on line is whitespace -- drop it, unless this
            # is the very beginning of the text (ie. no lines started yet).
            if self.drop_whitespace and chunks[-1].strip() == "" and len(lines) > 0:
                del chunks[-1]

            while len(chunks) > 0:
                length = len(chunks[-1])

                # Can at least squeeze this chunk onto the current line.
                if cur_len + length <= width:
                    cur_line.append(chunks.pop())
                    cur_len += length

                # Nope, this line is full.
                else:
                    break

            # The current line is full, and the next chunk is too big to
            # fit on *any* line (not just this one).
            if len(chunks) > 0 and len(chunks[-1]) > width:
                self._handle_long_word(chunks, cur_line, cur_len, width)
                cur_len = sum([len(chunk) for chunk in cur_line])

            # If the last chunk on this line is all whitespace, drop it.
            if self.drop_whitespace and len(cur_line) > 0 and cur_line[-1].strip() == "":
                cur_len -= len(cur_line[-1])
                del cur_line[-1]

            if len(cur_line) > 0:
                if (
                    max_lines is None
                    or len(lines) + 1 < max_lines
                    or (
                        len(chunks) == 0
                        or self.drop_whitespace
                        and len(chunks) == 1
                        and chunks[0].strip() == ""
                    )
                    and cur_len <= width
                ):
                    # Convert current line back to a string and store it in
                    # list of all lines (return value).
                    lines.append(indent + "".join(cur_line))
                else:
                    placed = False
                    while len(cur_line) > 0:
                        if cur_line[-1].strip() != "" and cur_len + len(self.placeholder) <= width:
                            cur_line.append(self.placeholder)
                            lines.append(indent + "".join(cur_line))
                            placed = True
                            break
                        cur_len -= len(cur_line[-1])
                        del cur_line[-1]
                    if not placed:
                        if len(lines) > 0:
                            prev_line = lines[-1].rstrip()
                            if len(prev_line) + len(self.placeholder) <= self.width:
                                lines[-1] = prev_line + self.placeholder
                                break
                        lines.append(indent + self.placeholder.lstrip())
                    break

        return lines

    def _split_chunks(self, text: str) -> list[str]:
        text = self._munge_whitespace(text)
        return self._split(text)

    def wrap(self, text: str) -> list[str]:
        """wrap(text : string) -> [string]

        Reformat the single paragraph in 'text' so it fits in lines of
        no more than 'self.width' columns, and return a list of wrapped
        lines.  Tabs in 'text' are expanded with string.expandtabs(),
        and all other whitespace characters (including newline) are
        converted to space.
        """
        chunks = self._split_chunks(text)
        if self.fix_sentence_endings:
            self._fix_sentence_endings(chunks)
        return self._wrap_chunks(chunks)

    def fill(self, text: str) -> str:
        """fill(text : string) -> string

        Reformat the single paragraph in 'text' to fit in lines of no
        more than 'self.width' columns, and return a new string
        containing the entire wrapped paragraph.
        """
        return "\n".join(self.wrap(text))


def _run_end(text: str, start: int, whitespace: bool) -> int:
    """Returns where the run of whitespace, or of other characters, that starts at `start`
    ends."""
    end = start + 1
    while end < len(text) and (text[end] in _whitespace) == whitespace:
        end += 1
    return end


def _is_word(text: str, place: int) -> bool:
    """Returns true when `text` has a character at `place` that `\\w` matches."""
    return 0 <= place < len(text) and (text[place].isalnum() or text[place] == "_")


def _is_letter(text: str, place: int) -> bool:
    """Returns true when `text` has a character at `place` that `[^\\d\\W]` matches."""
    return _is_word(text, place) and not text[place].isdecimal()


def _is_word_punct(text: str, place: int) -> bool:
    """Returns true when `text` has a character at `place` that `[\\w!"'&.,?]` matches."""
    return _is_word(text, place) or (0 <= place < len(text) and text[place] in "!\"'&.,?")


def _dashes(text: str, place: int) -> int:
    """Returns how many hyphens follow one another from `place`."""
    end = place
    while end < len(text) and text[end] == "-":
        end += 1
    return end - place


def _em_dash_follows(text: str, place: int) -> bool:
    """Returns true when two hyphens or more, then a word character, follow a word or a
    punctuation character at `place`: `(?<=wp)(?=-{2,}\\w)`."""
    dashes = _dashes(text, place)
    return _is_word_punct(text, place - 1) and dashes >= 2 and _is_word(text, place + dashes)


def _hyphen_breaks(text: str, place: int) -> bool:
    """Returns true when the hyphen at `place` joins letters, so a chunk ends after it:
    `-(?:(?<=lt{2}-)|(?<=lt-lt-))(?=lt-?lt)`."""
    before = (_is_letter(text, place - 2) and _is_letter(text, place - 1)) or (
        _is_letter(text, place - 3) and text[place - 2] == "-" and _is_letter(text, place - 1)
    )
    after = _is_letter(text, place + 1) and (
        (place + 2 < len(text) and text[place + 2] == "-" and _is_letter(text, place + 3))
        or _is_letter(text, place + 2)
    )
    return before and after


def _chunk_end(text: str, start: int) -> int:
    """Returns where the chunk that starts at `start` ends, as `wordsep_re` matches it: a run
    of whitespace, an em-dash between words, or a word, possibly hyphenated."""
    if text[start] in _whitespace:
        return _run_end(text, start, True)
    if _em_dash_follows(text, start):
        return start + _dashes(text, start)
    # The word takes as few characters as it can before one of the endings matches.
    end = start + 1
    while True:
        if end < len(text) and text[end] == "-" and _hyphen_breaks(text, end):
            return end + 1
        if end == len(text) or text[end] in _whitespace or _em_dash_follows(text, end):
            return end
        end += 1


def _ends_sentence(chunk: str) -> bool:
    """Returns true when `chunk` ends a sentence, as `[a-z][.!?]["']?\\z` finds."""
    end = len(chunk)
    if end > 0 and chunk[end - 1] in "\"'":
        end -= 1
    return end >= 2 and chunk[end - 1] in ".!?" and "a" <= chunk[end - 2] <= "z"


# -- Convenience interface ---------------------------------------------


def wrap(
    text: str,
    width: int = 70,
    *,
    initial_indent: str = "",
    subsequent_indent: str = "",
    expand_tabs: bool = True,
    tabsize: int = 8,
    replace_whitespace: bool = True,
    fix_sentence_endings: bool = False,
    break_long_words: bool = True,
    break_on_hyphens: bool = True,
    drop_whitespace: bool = True,
    max_lines: int | None = None,
    placeholder: str = " [...]",
) -> list[str]:
    """Wrap a single paragraph of text, returning a list of wrapped lines.

    Reformat the single paragraph in 'text' so it fits in lines of no
    more than 'width' columns, and return a list of wrapped lines.  By
    default, tabs in 'text' are expanded with string.expandtabs(), and
    all other whitespace characters (including newline) are converted to
    space.  See TextWrapper class for available keyword args to customize
    wrapping behaviour.
    """
    w = TextWrapper(
        width,
        initial_indent,
        subsequent_indent,
        expand_tabs,
        replace_whitespace,
        fix_sentence_endings,
        break_long_words,
        drop_whitespace,
        break_on_hyphens,
        tabsize,
        max_lines=max_lines,
        placeholder=placeholder,
    )
    return w.wrap(text)


def fill(
    text: str,
    width: int = 70,
    *,
    initial_indent: str = "",
    subsequent_indent: str = "",
    expand_tabs: bool = True,
    tabsize: int = 8,
    replace_whitespace: bool = True,
    fix_sentence_endings: bool = False,
    break_long_words: bool = True,
    break_on_hyphens: bool = True,
    drop_whitespace: bool = True,
    max_lines: int | None = None,
    placeholder: str = " [...]",
) -> str:
    """Fill a single paragraph of text, returning a new string.

    Reformat the single paragraph in 'text' to fit in lines of no more
    than 'width' columns, and return a new string containing the entire
    wrapped paragraph.  As with wrap(), tabs are expanded and other
    whitespace characters converted to space.  See TextWrapper class for
    available keyword args to customize wrapping behaviour.
    """
    w = TextWrapper(
        width,
        initial_indent,
        subsequent_indent,
        expand_tabs,
        replace_whitespace,
        fix_sentence_endings,
        break_long_words,
        drop_whitespace,
        break_on_hyphens,
        tabsize,
        max_lines=max_lines,
        placeholder=placeholder,
    )
    return w.fill(text)


def shorten(
    text: str,
    width: int,
    *,
    initial_indent: str = "",
    subsequent_indent: str = "",
    expand_tabs: bool = True,
    tabsize: int = 8,
    replace_whitespace: bool = True,
    fix_sentence_endings: bool = False,
    break_long_words: bool = True,
    break_on_hyphens: bool = True,
    drop_whitespace: bool = True,
    placeholder: str = " [...]",
) -> str:
    """Collapse and truncate the given text to fit in the given width.

    The text first has its whitespace collapsed.  If it then fits in
    the *width*, it is returned as is.  Otherwise, as many words
    as possible are joined and then the placeholder is appended::

        >>> textwrap.shorten("Hello  world!", width=12)
        'Hello world!'
        >>> textwrap.shorten("Hello  world!", width=11)
        'Hello [...]'
    """
    w = TextWrapper(
        width,
        initial_indent,
        subsequent_indent,
        expand_tabs,
        replace_whitespace,
        fix_sentence_endings,
        break_long_words,
        drop_whitespace,
        break_on_hyphens,
        tabsize,
        max_lines=1,
        placeholder=placeholder,
    )
    return w.fill(" ".join(text.strip().split()))


# -- Loosely related functionality -------------------------------------


def dedent(text: str) -> str:
    """Remove any common leading whitespace from every line in `text`.

    This can be used to make triple-quoted strings line up with the left
    edge of the display, while still presenting them in the source code
    in indented form.

    Note that tabs and spaces are both treated as whitespace, but they
    are not equal: the lines "  hello" and "\\thello" are
    considered to have no common leading whitespace.

    Entirely blank lines are normalized to a newline character.
    """
    lines = text.split("\n")

    # Get length of leading whitespace, inspired by ``os.path.commonprefix()``.
    non_blank_lines = [line for line in lines if len(line) > 0 and not line.isspace()]
    margin = 0
    if len(non_blank_lines) > 0:
        first = min(non_blank_lines)
        last = max(non_blank_lines)
        while margin < len(first) and first[margin] == last[margin] and first[margin] in " \t":
            margin += 1
    return "\n".join([line[margin:] if not line.isspace() else "" for line in lines])


def indent(text: str, prefix: str, predicate: Callable[[str], bool] | None = None) -> str:
    """Adds 'prefix' to the beginning of selected lines in 'text'.

    If 'predicate' is provided, 'prefix' will only be added to the lines
    where 'predicate(line)' is True. If 'predicate' is not provided,
    it will default to adding 'prefix' to all non-empty lines that do not
    consist solely of whitespace characters.
    """
    prefixed_lines: list[str] = []
    for line in text.splitlines(True):
        if (not line.isspace()) if predicate is None else predicate(line):
            prefixed_lines.append(prefix)
        prefixed_lines.append(line)
    return "".join(prefixed_lines)
