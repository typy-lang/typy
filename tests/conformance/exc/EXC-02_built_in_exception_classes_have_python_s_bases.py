# typy-test: EXC-02, EXC-01
# typy-expect: accepted
def describe(error: Exception) -> str:
    return f"{type(error).__name__}: {error}"


lookup: LookupError = KeyError("key")
arithmetic: ArithmeticError = ZeroDivisionError("zero")
value: ValueError = UnicodeError("text")
runtime: RuntimeError = RecursionError("deep")
print(describe(lookup), describe(arithmetic), describe(value), describe(runtime))
print(describe(IndexError(3)), describe(OverflowError()), describe(TypeError("t")))
print(describe(AssertionError("a")), describe(StopIteration()))
base: BaseException = Exception("root")
print(type(base), type(lookup))
