# typy-test: LIB-27
# typy-expect: accepted
# string gives CPython's constants, capwords and Template, whose placeholders substitute,
# stay or fail as CPython's do.
import string
from string import Template

print(string.ascii_letters, string.digits, string.hexdigits, string.octdigits)
print(string.punctuation, repr(string.whitespace), len(string.printable))
print(string.capwords(" hello  world  "), string.capwords("a-b-c", "-"), repr(string.capwords("")))
print(Template("$who likes $what").substitute(who="tim", what="kung pao"))
print(Template("${noun}ification $$ $x").safe_substitute({"noun": "fizz"}))
print(Template("$a ${b} $a").get_identifiers(), Template("$a and $").is_valid())
print(Template("$count items").substitute({"count": 3}), Template("$x$y").safe_substitute(x=1))
for text in ["$", "x $ y", "line\n${", "$1", "ab\ncd$!"]:
    try:
        Template(text).substitute({"a": 1})
    except ValueError as error:
        print(error)
    print(Template(text).is_valid(), repr(Template(text).safe_substitute(a=1)))
try:
    Template("$missing").substitute(other=1)
except KeyError as error:
    print(repr(error))
