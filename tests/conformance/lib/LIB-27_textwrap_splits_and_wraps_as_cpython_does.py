# typy-test: LIB-27
# typy-expect: accepted
# textwrap splits text into words, whitespace and parts of hyphenated words as CPython's
# regular expression does, then wraps, fills, shortens, dedents and indents it.
import textwrap

text = "The well-known rule:\twrap at word boundaries -- never mid-word. Done!  Yes."
print(textwrap.wrap(text, 18))
print(textwrap.wrap(text, 12, break_on_hyphens=False, break_long_words=False))
print(textwrap.fill(text, 24, initial_indent="* ", subsequent_indent="  ", fix_sentence_endings=True))
print(textwrap.wrap(text, 16, max_lines=2, placeholder=" ..."))
print(textwrap.shorten(text, 30), textwrap.shorten("Hello  world!", width=12))
wrapper = textwrap.TextWrapper(width=10, expand_tabs=False, replace_whitespace=False, drop_whitespace=False)
print(wrapper.wrap("a\tb  c-d-e supercalifragilistic"))
print(repr(textwrap.dedent("    def f():\n        return 1\n\n    f()\n")))
print(textwrap.indent("one\n\ntwo\n", "> "), end="")
print(textwrap.indent("one\ntwo\n", "+ ", lambda line: line.startswith("t")), end="")
for width, holder in ((0, " [...]"), (3, " [...]")):
    try:
        textwrap.wrap(text, width, max_lines=1, placeholder=holder)
    except ValueError as error:
        print("ValueError", error)
