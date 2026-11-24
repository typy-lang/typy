//! Tests of code generation through the public API: programs are parsed, checked, lowered and turned into LLVM IR and object code.

use camino::Utf8PathBuf;
use typy_check::TypeChecker;
use typy_codegen::{
    CodegenOptions, CodegenOutput, Optimization, Target, TargetTriple, assembly, llvm_ir, object,
    outputs,
};
use typy_source::{Project, ProjectSettings};
use typy_tyir::TyirModule;

fn compile(source: &str) -> (Project, TyirModule) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let directory = std::env::temp_dir().join(format!(
        "typy-codegen-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.clone()).unwrap();
    std::fs::write(root.join("app.py"), source).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root))
        .expect("the program parses");
    let program = TypeChecker::check(&project)
        .unwrap_or_else(|diagnostics| panic!("rejected: {diagnostics:?}"));
    let module =
        typy_lower::lower(&program).unwrap_or_else(|diagnostics| panic!("{diagnostics:?}"));
    let _ = std::fs::remove_dir_all(directory);
    (project, module)
}

const PROGRAMS: &[&str] = &[
    "print(1 + 2)\n",
    "def fib(n: int) -> int:\n    if n < 2:\n        return n\n    return fib(n - 1) + fib(n - 2)\nprint(fib(10))\n",
    "def greet(name: str, punctuation: str = \"!\") -> str:\n    return \"hello \" + name + punctuation\nprint(greet(\"world\"), greet(\"you\", \"?\"))\n",
    "total = 0\nfor i in range(10):\n    total += i\nwhile total > 100:\n    total = total // 2\nelse:\n    pass\nprint(total, sep=\", \", end=\"\\n\")\n",
    "print(len(\"abc\"), abs(-2.5), abs(-3), min(3, 1, 2), max(\"a\", \"b\"), int(\"12\"), float(3), str(1.5), bool(0))\n",
    "print(repr(\"x\"), ord(\"a\"), chr(98), \"abc\"[-1], 7 / 2, 2 ** 10, 1 < 2 <= 3, not True, 1.5 != 2.5)\n",
    "print(1 < 2.5, 2.5 >= 1, \"a\" < \"b\" < \"c\", True == False, 1 != 2, not 1, not \"\", 1 and 2, \"\" or \"x\")\n",
    "y = 1.5\ny /= 2.0\ny += 1.0\ns = \"a\"\ns += \"\\u00e9\\u03b1\\U0001f600\"\ns *= 2\nprint(y, s, -y, 123456789012345678901234567890)\n",
    "def f(flag: bool = True, ratio: float = 0.5, none: None = None) -> None:\n    return None\nf()\nx = 1\nprint(x is None, None is None)\n",
    "n = 3\nfor i in range(10, 0, -n):\n    if i == 4:\n        break\n    print(i)\nelse:\n    print(\"done\")\n",
    "x = 0\n",
    "def less(a: int, b: int) -> bool:\n    return a < b\npair = (less(1, 2), (\"x\", 2.5), None)\na, (b, c), d = pair\nprint(pair, a, b, c, d, pair[1][0], len(pair), pair == pair, pair < pair)\n",
    "",
    "class E:\n    pass\ne = E()\nprint(e == e, e != E(), e is e, bool(e), str(e) == repr(e), [e] == [e])\n",
    "class P:\n    \"\"\"A point.\"\"\"\n    def __init__(self, x: int, s: str) -> None:\n        self.x = x\n        self.s = s\n        self.none: None = None\n        self.items: list[int] = []\n        self.next: P | None = None\n    def add(self, v: int) -> None:\n        self.items += [v]\n        self.x += v\n        self.s *= 2\n    def __repr__(self) -> str:\n        return 'P' + str(self.x)\np = P(1, 'a')\np.add(2)\nq = P(2, 'b')\nq.next = p\nn = q.next\nprint(P(3, 'c').x, P(4, 'd').s, p.none, p.items, p, repr(q), [p, q], n is p, n is not None, p is q)\n",
    "class R:\n    def __init__(self, n: int) -> None:\n        self.n = n\n    def __str__(self) -> str:\n        return str(10 // self.n)\n    def __repr__(self) -> str:\n        return 'R' + str(10 // self.n)\nr = R(2)\nprint(r, repr(r), [r], {r: r.n})\nr.n = 0\nprint(r)\n",
    "class T:\n    def __init__(self, pair: tuple[int, str], flag: bool, ratio: float) -> None:\n        self.pair = pair\n        self.flag = flag\n        self.ratio = ratio\n        self.maybe: bool | None = None\nt = T((1, 'x'), True, 0.5)\nt.maybe = False\nt.ratio /= 2.0\nprint(t.pair, t.flag, t.ratio, t.maybe, {t} == {t})\n",
    "class P:\n    pass\nx: int | None = None\nkinds = [int, P, type(P()), type(x)]\nprint(kinds, type(1) == int, int != str, P.__name__, type(x).__name__, bool(int), {str: 1}[type('a')], type({1: 1}.values()), object, type(int))\n",
    "class P:\n    pass\nx = 1\nf = 2.5\nb = True\ns = 'h\\u00e9'\nm: int | None = None\nn: int | None = 4\nitems = [1]\nprint(f'', f'{x}', f'{x:>5}', f'{f:.2f}', f'{b}', f'{b:d}', f'{s:^7}', f'{m}', f'{m:>4}', f'{n:03}', f'{items}', f'{items:}', f'{P():}', f'{x!s}', f'{s!r:>8}', f'{s!a}', f'{x = }', f'{f=:.1e}', f'{x:{x}}', f'{{}}')\n",
    "def check(n: int) -> int:\n    if n < 0:\n        raise ValueError('negative', n, [n])\n    return n\ne: LookupError = KeyError('k')\nprint(e, repr(e), f'{e}', type(e), [e], check(1))\nraise RuntimeError('r') from e\n",
    "def fail() -> str:\n    raise TypeError from None\nraise NotImplementedError\n",
    "def sign(b: bool) -> int:\n    if b:\n        return 1\n    else:\n        return -1\nprint(sign(True), sign(False))\n",
    "def first(items: list[int]) -> int:\n    try:\n        return items[0]\n    except KeyError:\n        return -1\n    except LookupError as e:\n        print(repr(e))\n    return 0\ntotal = 0\nfor n in [1, 0, 2]:\n    try:\n        total += 10 // n\n    except ZeroDivisionError as e:\n        print(e)\n        continue\n    except:\n        break\n    else:\n        print(total)\ntry:\n    print(first([]))\nexcept ValueError:\n    raise\n",
    "def label(n: int) -> str:\n    text = str(n)\n    try:\n        if n > 1:\n            return text * 2\n        print(10 // n)\n    except ZeroDivisionError as e:\n        return str(e)\n    finally:\n        print('done', n)\n    return text\nfor n in range(4):\n    try:\n        if n == 1:\n            continue\n        if n == 3:\n            break\n    finally:\n        print(label(n))\ntry:\n    raise KeyError('k')\nfinally:\n    raise RuntimeError('r')\n",
    "class AppError(Exception):\n    pass\n\n\nclass ConfigError(AppError):\n    def __init__(self, key: str, line: int) -> None:\n        super().__init__(f\"bad key {key}\")\n        self.key = key\n        self.line = line\n\n    def where(self) -> str:\n        return f\"{self.key} at line {self.line}\"\n\n\nclass MissingKey(ConfigError):\n    pass\n\n\nclass Quiet(ValueError):\n    def __str__(self) -> str:\n        return \"quiet!\"\n\n\ndef load(key: str) -> int:\n    if key == \"port\":\n        raise MissingKey(key, 7)\n    if key == \"host\":\n        raise Quiet(\"host\", 1)\n    if key == \"app\":\n        raise AppError\n    return 1\n\n\nfor key in [\"port\", \"host\", \"app\", \"user\"]:\n    try:\n        print(key, load(key))\n    except ConfigError as error:\n        print(\"config:\", error, repr(error), error.where(), error.key, type(error).__name__)\n    except ValueError as error:\n        print(\"value:\", error, repr(error), type(error).__name__)\n    except AppError as error:\n        print(\"app:\", repr(error), str(error) == \"\")\ne = MissingKey(\"k\", 2)\nprint(e.line)\nraise MissingKey(\"timeout\", 3)\n",
    "class HttpError(Exception):\n    def __init__(self, status: int, reason: str) -> None:\n        super().__init__(status, reason)\n        self.status = status\n        self.reason = reason\n\n\nclass RetryableError(HttpError):\n    def __init__(self, status: int, reason: str, delay: float) -> None:\n        super().__init__(status, reason)\n        self.delay = delay\n        self.attempts: list[int] = []\n\n\nerrors: list[HttpError] = [HttpError(404, \"missing\"), RetryableError(503, \"busy\", 1.5)]\nfor error in errors:\n    print(error.status, error.reason, repr(error), str(error))\nretry = RetryableError(429, \"slow down\", 0.25)\nretry.attempts.append(1)\nretry.status += 1\nprint(retry.status, retry.delay, retry.attempts, retry)\ntry:\n    raise retry\nexcept HttpError as caught:\n    print(\"caught\", caught.status, type(caught).__name__)\n",
    "class Coded(Exception):\n    def __init__(self, code: int = 1) -> None:\n        super().__init__()\n        self.code = code\ne = Coded()\nf = Coded(code=2)\nprint(e.code, f.code, repr(e))\ndef g() -> int:\n    try:\n        return 1\n    finally:\n        raise ValueError('in finally')\nprint(g())\n",
    "def first(items: list[int] | None) -> int:\n    assert items is not None, 'required'\n    assert len(items) > 0\n    return items[0]\nprint(first([3]), first([]) if False else 0)\nassert first([1]) == 2, ('mismatch', 2)\n",
    "class Countdown:\n    def __init__(self, start: int) -> None:\n        self.current = start\n\n    def __iter__(self) -> \"Countdown\":\n        return self\n\n    def __next__(self) -> int:\n        if self.current <= 0:\n            raise StopIteration\n        self.current -= 1\n        return self.current + 1\n\n\nclass WordIterator:\n    def __init__(self, words: list[str]) -> None:\n        self.words = words\n        self.index = 0\n\n    def __iter__(self) -> \"WordIterator\":\n        return self\n\n    def __next__(self) -> str:\n        if self.index >= len(self.words):\n            raise StopIteration()\n        word = self.words[self.index]\n        self.index += 1\n        return word\n\n\nclass Sentence:\n    def __init__(self, text: str) -> None:\n        self.words = text.split()\n\n    def __iter__(self) -> WordIterator:\n        return WordIterator(self.words)\n\n\nclass Faulty:\n    def __iter__(self) -> \"Faulty\":\n        return self\n\n    def __next__(self) -> int:\n        raise ValueError(\"broken\")\n\n\nfor n in Countdown(3):\n    print(n)\nelse:\n    print(\"lift off\")\nsentence = Sentence(\"the quick brown fox\")\nfor word in sentence:\n    if word == \"brown\":\n        break\n    print(word)\nprint([w.upper() for w in sentence])\nfor i, w in enumerate(Sentence(\"a b\")):\n    print(i, w)\nfor n, w in zip(Countdown(5), Sentence(\"x y z\")):\n    print(n, w)\nfor outer in Countdown(2):\n    for inner in Countdown(outer):\n        print(outer, inner)\ntry:\n    for value in Faulty():\n        print(value)\nexcept ValueError as error:\n    print(\"caught\", error)\nfor value in Faulty():\n    print(value)\n",
    "items = [1, 2, 3]\nit = iter(items)\nprint(next(it), next(it))\nitems.append(4)\nprint(list(it), next(it, -1))\nchars = iter(\"h\u{e9}llo\")\nprint(next(chars), next(chars), [c for c in chars], type(chars).__name__, type(iter(\"ab\")).__name__)\nscores = {\"a\": 1, \"b\": 2}\nkeys = iter(scores)\nvalues = iter(scores.values())\npairs = iter(scores.items())\nprint(next(keys), next(values), next(pairs), list(pairs))\nprint(next(iter(b\"\\x05\")), next(iter({7})), next(iter((1, 2))), type(iter(scores.items())).__name__)\nempty: list[str] = []\nprint(next(iter(empty), None), next(iter(empty), \"nothing\"))\nfor n in iter([10, 20]):\n    print(n)\nagain = iter([1])\nprint(iter(again) is again)\ngrowing = {1: 1}\ntry:\n    for key in iter(growing):\n        growing[key + 1] = 0\nexcept RuntimeError as error:\n    print(\"error:\", error)\ntry:\n    next(iter(empty))\nexcept StopIteration as error:\n    print(\"stop:\", repr(error))\nnext(iter(\"\"))\n",
    "class Resource:\n    def __init__(self, name: str, swallow: bool = False) -> None:\n        self.name = name\n        self.swallow = swallow\n\n    def __enter__(self) -> \"Resource\":\n        print(\"enter\", self.name)\n        return self\n\n    def __exit__(self, kind, error, trace) -> bool:\n        if error is None:\n            print(\"exit\", self.name, kind, error, trace is None)\n        else:\n            print(\"exit\", self.name, kind, repr(error), trace is None)\n        return self.swallow\n\n\nclass Pair:\n    def __enter__(self) -> tuple[int, str]:\n        return (1, \"one\")\n\n    def __exit__(self, kind, error, trace) -> None:\n        print(\"pair done\")\n\n\ndef early(n: int) -> int:\n    with Resource(\"early\"):\n        if n > 0:\n            return n * 10\n    return -1\n\n\nwith Resource(\"plain\") as resource:\n    print(\"body\", resource.name)\nprint(early(2), early(0))\nfor i in range(3):\n    with Resource(f\"loop{i}\"):\n        if i == 0:\n            continue\n        if i == 2:\n            break\n        print(\"loop body\", i)\nwith Resource(\"quiet\", True):\n    raise KeyError(\"swallowed\")\nprint(\"after quiet\")\nwith Resource(\"outer\"), Resource(\"inner\", True) as inner:\n    print(\"both\", inner.name)\n    raise ValueError(\"inner swallows\")\nwith Pair() as (number, word):\n    print(number, word)\ntry:\n    with Resource(\"loud\"):\n        raise IndexError(\"passes\")\nexcept IndexError as error:\n    print(\"caught\", error)\n",
    "class Failing:\n    def __enter__(self) -> None:\n        return None\n\n    def __exit__(self, kind, error, trace) -> None:\n        raise RuntimeError(\"exit failed\")\n\n\nwith Failing():\n    raise ValueError(\"body failed\")\n",
    "numbers = list(range(10))\ndel numbers[0]\ndel numbers[-1]\nprint(numbers)\ndel numbers[1:3]\nprint(numbers)\ndel numbers[::2]\nprint(numbers)\ndel numbers[10:20], numbers[0]\nprint(numbers)\nwords = [\"a\", \"b\", \"c\", \"d\"]\ndel words[::-1]\nprint(words)\npairs = [(1, \"one\"), (2, \"two\"), (3, \"three\")]\ndel pairs[1]\nprint(pairs)\nscores = {\"ann\": 3, \"bob\": 5}\ndel scores[\"ann\"]\nprint(scores)\nnested = {\"k\": [1, 2, 3]}\ndel nested[\"k\"][1]\nprint(nested)\ntry:\n    del numbers[100]\nexcept IndexError as error:\n    print(\"index:\", error)\ntry:\n    del scores[\"zed\"]\nexcept KeyError as error:\n    print(\"key:\", repr(error))\ntry:\n    del words[::0]\nexcept ValueError as error:\n    print(\"value:\", error)\ndel scores[\"bob\"], scores[\"bob\"]\n",
    "letters = [\"a\", \"b\", \"c\", \"d\", \"e\"]\nletters[1:3] = [\"X\"]\nprint(letters)\nletters[:0] = [\"start\", \"again\"]\nprint(letters)\nletters[::2] = [\"1\", \"2\", \"3\"]\nprint(letters)\nletters[:] = []\nprint(letters)\nnumbers = [1, 2, 3]\nnumbers[1:] = numbers\nprint(numbers)\ngrid = [[0, 0], [1, 1]]\ngrid[0][:] = [5, 6]\nprint(grid)\npairs = [(1, \"a\"), (2, \"b\")]\npairs[1:] = [(3, \"c\"), (4, \"d\")]\nprint(pairs)\ntry:\n    numbers[::2] = [0]\nexcept ValueError as error:\n    print(error)\nnumbers[::-1] = [9, 8]\n",
];

#[test]
fn every_program_generates_valid_llvm_ir_at_every_level() {
    for source in PROGRAMS {
        let (project, module) = compile(source);
        for optimization in Optimization::ALL {
            let options = CodegenOptions::new(optimization);
            let ir = llvm_ir(&module, Target::host(), &options, &project)
                .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}\n{source}"));
            assert!(
                ir.contains("define ") && ir.contains("@__typy_main"),
                "{source}\n{ir}"
            );
        }
    }
}

#[test]
fn cli_29_outputs_from_one_module_match_the_outputs_generated_alone() {
    let (project, module) = compile(PROGRAMS[1]);
    let options = CodegenOptions::new(Optimization::Speed);
    let host = Target::host();
    let [unoptimised, ir, bitcode, asm, obj]: [Vec<u8>; 5] = outputs(
        &module,
        host,
        &options,
        &project,
        &[
            CodegenOutput::UnoptimisedIr,
            CodegenOutput::LlvmIr,
            CodegenOutput::Bitcode,
            CodegenOutput::Assembly,
            CodegenOutput::Object,
        ],
    )
    .unwrap()
    .try_into()
    .unwrap();
    // Writing the assembly first must leave the module the object is written from unchanged.
    assert_eq!(
        obj,
        object(&module, host, &options, &project).unwrap().bytes
    );
    assert_eq!(
        asm,
        assembly(&module, host, &options, &project)
            .unwrap()
            .into_bytes()
    );
    assert_eq!(
        ir,
        llvm_ir(&module, host, &options, &project)
            .unwrap()
            .into_bytes()
    );
    // LLVM wraps the bitcode of an Apple target in a header of its own, as clang does.
    let magic: &[u8] = if cfg!(target_os = "macos") {
        &[0xde, 0xc0, 0x17, 0x0b]
    } else {
        b"BC\xc0\xde"
    };
    assert_eq!(&bitcode[..4], magic);
    assert_ne!(unoptimised, ir);
    let only_unoptimised = outputs(
        &module,
        host,
        &options,
        &project,
        &[CodegenOutput::UnoptimisedIr],
    );
    assert_eq!(only_unoptimised.unwrap(), [unoptimised]);
}

#[test]
fn object_files_have_the_hosts_format_and_are_deterministic() {
    let (project, module) = compile(PROGRAMS[1]);
    let first = object(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::Speed),
        &project,
    )
    .unwrap();
    let second = object(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::Speed),
        &project,
    )
    .unwrap();
    let magic: &[u8] = if cfg!(target_os = "macos") {
        b"\xcf\xfa\xed\xfe"
    } else {
        b"\x7fELF"
    };
    assert_eq!(&first.bytes[..4], magic);
    assert_eq!(first, second);
    assert_eq!(first.target, Target::host());
}

#[test]
fn every_program_is_a_deterministic_mach_o_object_for_macos() {
    let target = Target {
        triple: TargetTriple::Aarch64AppleDarwin,
    };
    for source in PROGRAMS {
        let (project, module) = compile(source);
        let first = object(
            &module,
            target,
            &CodegenOptions::new(Optimization::Speed),
            &project,
        )
        .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}\n{source}"));
        let second = object(
            &module,
            target,
            &CodegenOptions::new(Optimization::Speed),
            &project,
        )
        .unwrap();
        // `MH_MAGIC_64`, then `CPU_TYPE_ARM64`, both little-endian.
        assert_eq!(
            &first.bytes[..8],
            b"\xcf\xfa\xed\xfe\x0c\x00\x00\x01",
            "{source}"
        );
        assert_eq!(first, second, "{source}");
    }
    let (project, module) = compile(PROGRAMS[1]);
    let ir = llvm_ir(
        &module,
        target,
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    assert!(
        ir.contains("target triple = \"arm64-apple-macosx11.0.0\""),
        "{ir}"
    );
}

/// Returns the type of each argument of the call of `name` in `ir`, without its value.
fn call_arguments(ir: &str, name: &str) -> Vec<String> {
    let line = ir
        .lines()
        .find(|line| line.contains(&format!(" @{name}(")) && line.contains("call "))
        .unwrap_or_else(|| panic!("no call of {name}\n{ir}"));
    let start = line.find(&format!("@{name}(")).unwrap() + name.len() + 2;
    let end = line.rfind(')').unwrap();
    line[start..end]
        .split(", ")
        .map(|argument| {
            let words: Vec<&str> = argument.split(' ').collect();
            words[..words.len() - 1].join(" ")
        })
        .collect()
}

#[test]
fn ffi_13_an_integer_on_apples_stack_takes_its_c_width_and_words_go_in_registers() {
    let (project, module) = compile(
        "import ctypes\n\nlib = ctypes.CDLL(\"libnarrow.so\")\nL = ctypes.c_long\n\
         lib.f.argtypes = [L, L, L, L, L, L, L, ctypes.c_double, ctypes.c_byte, \
         ctypes.c_ushort, ctypes.c_bool, ctypes.c_int, L]\nlib.f.restype = L\n\
         print(lib.f(1, 2, 3, 4, 5, 6, 7, 0.5, -1, 2, True, 3, 4))\n",
    );
    let words = ["i64"; 7].map(str::to_owned);
    for (triple, stack) in [
        (
            TargetTriple::Aarch64AppleDarwin,
            ["i64", "i16 zeroext", "i1 zeroext", "i32", "i64"],
        ),
        (TargetTriple::Aarch64LinuxGnu, ["i64"; 5]),
        (TargetTriple::X86_64LinuxGnu, ["i64"; 5]),
    ] {
        let ir = llvm_ir(
            &module,
            Target { triple },
            &CodegenOptions::new(Optimization::None),
            &project,
        )
        .unwrap();
        let mut expected = words.to_vec();
        expected.push("double".to_owned());
        expected.extend(stack.map(str::to_owned));
        assert_eq!(call_arguments(&ir, "f"), expected, "{triple}");
    }
}

/// A class whose attributes mix every alignment, declared in an order that pads.
const MIXED_CLASS: &str = "\
class Node:
    def __init__(self, key: int, red: bool, weight: float, seen: bool, size: int, \
parent: \"Node | None\") -> None:
        self.key = key
        self.red = red
        self.weight = weight
        self.seen = seen
        self.size = size
        self.parent = parent


n = Node(1, True, 0.5, False, 3, None)
print(n.key, n.red, n.weight, n.seen, n.size, n.parent is None)
";

#[test]
fn mem_11_an_instance_stores_its_attributes_by_alignment_and_its_descriptor_keeps_their_order() {
    let (project, module) = compile(MIXED_CLASS);
    for target in [
        Target::host(),
        Target {
            triple: TargetTriple::Aarch64AppleDarwin,
        },
    ] {
        let ir = llvm_ir(
            &module,
            target,
            &CodegenOptions::new(Optimization::None),
            &project,
        )
        .unwrap();
        assert!(
            ir.contains("{ i64, ptr, { i64, double, i64, ptr, i1, i1 } }"),
            "{ir}"
        );
        // In the declared order, each with its offset from the ordering rule: `key` 0,
        // `red` 32, `weight` 8, `seen` 33, `size` 16 and `parent` 24, of kinds int, bool,
        // float, bool, int and optional object.
        let fields = "[{ i32, i32, ptr } zeroinitializer, \
            { i32, i32, ptr } { i32 2, i32 32, ptr null }, \
            { i32, i32, ptr } { i32 1, i32 8, ptr null }, \
            { i32, i32, ptr } { i32 2, i32 33, ptr null }, \
            { i32, i32, ptr } { i32 0, i32 16, ptr null }, \
            { i32, i32, ptr } { i32 9, i32 24, ptr null }]";
        assert!(ir.contains(fields), "{ir}");
        assert!(
            ir.contains("{ i64 40, i64 6, ptr @typy.tuple0.fields }"),
            "{ir}"
        );
    }
}

#[test]
fn nfr_18_an_instance_takes_its_prefix_and_its_attributes_rounded_to_eight_bytes() {
    let (project, module) = compile(MIXED_CLASS);
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    // Eight bytes each for two `int`s, a `float` and an object, one each for two `bool`s.
    let attributes: u64 = 4 * 8 + 2;
    let bound = attributes.next_multiple_of(8);
    let size: u64 = ir
        .lines()
        .find_map(|line| {
            line.strip_prefix(
                "@typy.tuple0 = private unnamed_addr constant { i64, i64, ptr } { i64 ",
            )
        })
        .and_then(|rest| rest.split(',').next())
        .and_then(|size| size.parse().ok())
        .unwrap();
    assert!(size <= bound, "{size} > {bound}");
    assert_eq!(16 + size, 56);
}

/// A function that cannot raise and one that may, each called through a value.
const FUNCTION_VALUES: &str = "\
from typing import Callable


def double(value: int) -> int:
    return value * 2


def fail(value: int) -> int:
    raise ValueError(\"no\")


def apply(function: Callable[[int], int], value: int) -> int:
    return function(value)


print(apply(double, 2), apply(fail, 3))
";

#[test]
fn fun_05_a_function_value_is_an_immortal_object_of_its_descriptor() {
    let (project, module) = compile(FUNCTION_VALUES);
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    // The header is immortal and of the function type ID, then the descriptor: the name, the
    // qualified name, the code, no captures and its own address as the identity it stands
    // for (ADR-0076).
    assert!(
        ir.contains(
            "@typy.function.0 = private constant { i32, i32, ptr } \
             { i32 -1, i32 24, ptr @typy.function.descriptor.0 }"
        ),
        "{ir}"
    );
    assert!(
        ir.contains(
            "@typy.function.descriptor.0 = private unnamed_addr constant \
             { ptr, ptr, ptr, ptr, ptr } { ptr @typy.str.1, ptr @typy.str.1, \
             ptr @typy.thunk._TY8__main__6double, ptr null, ptr @typy.function.descriptor.0 }"
        ),
        "{ir}"
    );
    assert!(
        ir.contains("call { i64, ptr } @_TY8__main__5apply(ptr @typy.function.1, i64 7)"),
        "{ir}"
    );
}

#[test]
fn fun_08_the_hidden_functions_of_one_builtin_share_its_identity() {
    let (project, module) = compile(
        "from collections.abc import Callable\n\
         measure: Callable[[str], int] = len\n\
         count: Callable[[list[int]], int] = len\n\
         print(measure is count)\n",
    );
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    let descriptors: Vec<&str> = ir
        .lines()
        .filter(|line| line.starts_with("@typy.function.descriptor."))
        .collect();
    assert_eq!(descriptors.len(), 2, "{ir}");
    // The first descriptor of `len` names itself, and the second names the first.
    let first = descriptors[0].split(' ').next().unwrap();
    for descriptor in &descriptors {
        assert!(
            descriptor.contains(&format!("ptr null, ptr {first} }}")),
            "{descriptor}"
        );
    }
    // Each object is of the type ID of a built-in function.
    assert_eq!(
        ir.matches("{ i32 -1, i32 28, ptr @typy.function.descriptor.")
            .count(),
        2
    );
    assert!(ir.contains("call i8 @typy_rt_same_function("), "{ir}");
}

#[test]
fn fun_05_a_thunk_gives_every_function_the_raising_form_a_call_through_a_value_takes() {
    let (project, module) = compile(FUNCTION_VALUES);
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    let thunk = |name: &str| -> String {
        let start = ir.find(&format!("@typy.thunk.{name}(")).unwrap();
        ir[start..].split("\n}").next().unwrap().to_owned()
    };
    // A function that cannot raise has its result paired with no exception.
    let double = thunk("_TY8__main__6double");
    assert!(
        double.contains("call i64 @_TY8__main__6double(i64 %1)"),
        "{double}"
    );
    assert!(
        double.contains("insertvalue { i64, ptr } %3, ptr null, 1"),
        "{double}"
    );
    // One that may raise already returns the pair, so its thunk only calls it.
    let fail = thunk("_TY8__main__4fail");
    assert!(
        fail.contains("tail call { i64, ptr } @_TY8__main__4fail(i64 %1)"),
        "{fail}"
    );
    // The call through the value reads the code from the descriptor and passes the object.
    let start = ir.find("@_TY8__main__5apply(ptr %0, i64 %1)").unwrap();
    let apply = ir[start..].split("\n}").next().unwrap();
    assert!(
        apply
            .lines()
            .any(|line| line.contains("= call { i64, ptr } %") && line.contains("(ptr %")),
        "{apply}"
    );
}

/// A method a subclass overrides, read through an instance of the base class.
const BOUND_METHODS: &str = "\
from typing import Callable


class Counter:
    def __init__(self) -> None:
        self.count = 0

    def add(self, step: int) -> int:
        self.count += step
        return self.count


class Double(Counter):
    def add(self, step: int) -> int:
        self.count += 2 * step
        return self.count


def run(counter: Counter) -> Callable[[int], int]:
    return counter.add


print(run(Double())(1))
";

#[test]
fn fun_05_a_bound_method_chooses_its_classs_descriptor_and_its_thunk_reads_the_instance() {
    let (project, module) = compile(BOUND_METHODS);
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    // One descriptor for each method, both named `add`, of their own qualified names.
    for (method, symbol) in [
        ("Counter", "_TY8__main__11Counter.add"),
        ("Double", "_TY8__main__10Double.add"),
    ] {
        assert!(
            ir.lines()
                .any(|line| line.starts_with("@typy.method.descriptor.")
                    && line.contains(&format!("ptr @typy.method_thunk.{symbol}, ptr null, "))),
            "{method}: {ir}"
        );
        let start = ir
            .find(&format!("@typy.method_thunk.{symbol}(ptr %0, i64 %1) {{"))
            .unwrap();
        let thunk = ir[start..].split("\n}").next().unwrap();
        // The instance is the bound method's fourth field, passed first.
        assert!(
            thunk.contains(
                "getelementptr inbounds nuw { i32, i32, ptr, ptr }, ptr %0, i32 0, i32 3"
            ),
            "{thunk}"
        );
        assert!(
            thunk.contains(&format!("call i64 @{symbol}(ptr %3, i64 %1)")),
            "{thunk}"
        );
    }
    assert!(ir.contains("call ptr @typy_rt_method_new(i64 %"), "{ir}");
}

/// A nested function that captures an `int` and a `str`.
const CLOSURE: &str = "\
from typing import Callable


def make_adder(step: int, label: str) -> Callable[[int], int]:
    def add(value: int) -> int:
        print(label)
        return value + step
    return add


print(make_adder(1, \"x\")(2))
";

#[test]
fn fun_04_a_closure_comes_from_the_runtime_with_its_descriptor_and_the_layout_of_its_captures() {
    let (project, module) = compile(CLOSURE);
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    let symbol = "\"_TY8__main__23make_adder.<locals>.add\"";
    // The captures, `label` then `step`, are laid out by the descriptor the runtime releases
    // them through: an object at 0, an `int` at 8.
    assert!(
        ir.contains(
            "[{ i32, i32, ptr } { i32 3, i32 0, ptr null }, { i32, i32, ptr } { i32 0, i32 8, ptr null }]"
        ),
        "{ir}"
    );
    let descriptor = ir
        .lines()
        .find(|line| line.starts_with("@typy.closure.descriptor."))
        .unwrap();
    assert!(
        descriptor.contains(
            "ptr @\"typy.closure_thunk._TY8__main__23make_adder.<locals>.add\", ptr @typy.tuple0, "
        ),
        "{descriptor}"
    );
    assert!(
        ir.contains("call ptr @typy_rt_closure_new(ptr @typy.closure.descriptor."),
        "{ir}"
    );
    // The thunk passes the object on, which the nested function reads its captures from.
    let start = ir
        .find("@\"typy.closure_thunk._TY8__main__23make_adder.<locals>.add\"(ptr %0, i64 %1) {")
        .unwrap();
    let thunk = ir[start..].split("\n}").next().unwrap();
    assert!(
        thunk.contains(&format!("call i64 @{symbol}(ptr %0, i64 %1)")),
        "{thunk}"
    );
}

#[test]
fn fun_04_a_cell_comes_from_the_runtime_with_the_layout_of_its_value_and_flag() {
    let (project, module) = compile(
        "def outer() -> int:\n    total = 1\n    def read() -> int:\n        return total\n    \
         total = 2\n    return read()\n\n\nprint(outer())\n",
    );
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    // The layout of an `int` and its flag: the word at 0, the flag a `bool` at 8.
    assert!(
        ir.contains(
            "[{ i32, i32, ptr } zeroinitializer, { i32, i32, ptr } { i32 2, i32 8, ptr null }]"
        ),
        "{ir}"
    );
    assert!(
        ir.contains("call ptr @typy_rt_cell_new(ptr @typy.tuple"),
        "{ir}"
    );
    assert!(
        ir.contains("call ptr @typy_rt_free_variable_unbound(ptr @typy.str."),
        "{ir}"
    );
}

#[test]
fn the_entry_is_the_only_external_function() {
    let (project, module) = compile(PROGRAMS[1]);
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    assert!(ir.contains("define ptr @__typy_main()"), "{ir}");
    assert!(
        ir.contains("define internal { i64, ptr } @_TY8__main__3fib(i64"),
        "{ir}"
    );
}

#[test]
fn raising_calls_add_a_traceback_entry_with_the_line() {
    let (project, module) = compile("def f(n: int) -> int:\n    return 10 // n\nprint(f(0))\n");
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    assert!(ir.contains("call void @typy_rt_traceback_add("), "{ir}");
    assert!(ir.contains("i32 2)"), "the division is on line 2:\n{ir}");
    assert!(ir.contains("c\"app.py\\00\""), "{ir}");
}

#[test]
fn constants_are_immortal_static_objects() {
    let (project, module) = compile("print(\"hi\", 123456789012345678901234567890)\n");
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    assert!(ir.contains("private unnamed_addr constant { i32, i32, i64, i32, i32, i64, [3 x i8] } { i32 -1, i32 1, i64 2, i32 1, i32 0, i64 -1, [3 x i8] c\"hi\\00\" }"), "{ir}");
    assert!(
        ir.contains("{ i32 -1, i32 3, i32 2, i32 0, [2 x i64]"),
        "{ir}"
    );
}

#[test]
fn assembly_is_text_for_the_target() {
    let (project, module) = compile("print(1)\n");
    let text = assembly(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::Speed),
        &project,
    )
    .unwrap();
    assert!(text.contains("__typy_main"), "{text}");
}

#[test]
fn release_small_marks_every_function_minsize() {
    let (project, module) = compile(PROGRAMS[1]);
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::Size),
        &project,
    )
    .unwrap();
    assert!(ir.contains("minsize") && ir.contains("optsize"), "{ir}");
}

#[test]
fn reading_an_item_of_a_value_that_is_not_a_tuple_is_an_internal_error() {
    let (project, mut module) = compile("x = 0\n");
    let entry = module.entry.0 as usize;
    let function = &mut module.functions[entry];
    let item = typy_tyir::ValueId(u32::try_from(function.values.len()).unwrap());
    function.values.push(typy_tyir::ValueInfo {
        ty: typy_tyir::TyirType::Int,
        representation: typy_tyir::ValueRepresentation::Inline,
    });
    let span = function.blocks[0].instructions[0].span;
    function.blocks[0]
        .instructions
        .push(typy_tyir::Instruction {
            result: Some(item),
            operation: typy_tyir::Operation::TupleItem {
                tuple: typy_tyir::ValueId(0),
                index: 0,
            },
            span,
        });
    let error = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap_err();
    let text = format!("{error:?}");
    assert!(text.contains("is not a tuple"), "{text}");
}

#[test]
fn counting_attributes_placeholders_and_descriptors_reach_the_ir() {
    let (project, module) = compile(concat!(
        "class P:\n",
        "    def __init__(self, x: int) -> None:\n",
        "        self.x = x\n",
        "\n\n",
        "def run() -> None:\n",
        "    p = P(3)\n",
        "    p.x = 7\n",
        "    s = \"a\" + \"b\"\n",
        "    t = (s, 1, (s, 2.5))\n",
        "    u = (s, 1, (s, 2.5))\n",
        "    q = P(4)\n",
        "    pairs = [t, u]\n",
        "    items = {\"k\"}\n",
        "    table = {\"k\": 1}\n",
        "    print(p.x, q.x, pairs, items, table)\n",
        "\n\n",
        "run()\n",
    ));
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    for expected in [
        "call void @typy_rt_increment(",
        "call void @typy_rt_decrement(",
        "call void @typy_rt_int_increment(",
        "call void @typy_rt_int_decrement(",
        // `p.x = 7` stores the tagged word of 7.
        "store i64 15",
        "@typy_rt_placeholder_list",
        "@typy_rt_placeholder_set",
        "@typy_rt_placeholder_dict",
        // A `str`, an `int` and a nested tuple, then a `str` and a `float`.
        "{ i32 3, i32 0, ptr null }, { i32, i32, ptr } { i32 0, i32 8, ptr null }, { i32, i32, ptr } { i32 5, i32 16, ptr @typy.tuple0 }",
        "{ i32 3, i32 0, ptr null }, { i32, i32, ptr } { i32 1, i32 8, ptr null }",
    ] {
        assert!(ir.contains(expected), "{expected:?} missing from\n{ir}");
    }
    let defined = |name: &str| {
        ir.lines()
            .filter(|line| line.starts_with(&format!("{name} = ")))
            .count()
    };
    // Two uses of one tuple type and two instances of one class share their statics.
    assert_eq!(defined("@typy.tuple0"), 1, "{ir}");
    assert_eq!(defined("@typy.tuple1"), 1, "{ir}");
    assert_eq!(defined("@typy.class0"), 1, "{ir}");
    assert!(!ir.contains("@typy.tuple0.1"), "{ir}");
    assert!(!ir.contains("@typy.class0.1 "), "{ir}");
}

#[test]
fn an_instance_held_in_the_frame_is_made_in_a_block_released_before_each_return() {
    let source = "class P:\n    def __init__(self, x: int) -> None:\n        self.x = x\n\n\ndef f(n: int) -> int:\n    p = P(n)\n    if n > 0:\n        return p.x\n    return 0\n\n\nprint(f(1))\n";
    let (project, module) = compile(source);
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    let start = ir.find("@_TY8__main__1f(").expect("f is defined");
    let body = &ir[start..start + ir[start..].find("\n}\n").expect("f ends")];
    // The header and the class take two words and the one `int` attribute a third.
    assert!(body.contains("alloca [3 x i64], align 16"), "{body}");
    assert!(body.contains("store i64 0, ptr"), "{body}");
    assert_eq!(
        body.matches("call ptr @typy_rt_instance_on_stack(").count(),
        1,
        "{body}"
    );
    let returns = body.matches("\n  ret ").count();
    assert!(returns > 0, "{body}");
    assert_eq!(
        body.matches("call void @typy_rt_instance_off_stack(")
            .count(),
        returns,
        "{body}"
    );
    assert!(!body.contains("@typy_rt_instance_new("), "{body}");
}

#[test]
fn every_object_defines_hidden_heap_hooks_over_the_c_library() {
    let (project, module) = compile("print([1] * 3)\n");
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    for (hook, call) in [
        (
            "define hidden ptr @typy_rt_heap_allocate(i64 %0)",
            "tail call ptr @malloc(i64 %0)",
        ),
        (
            "define hidden ptr @typy_rt_heap_reallocate(ptr %0, i64 %1)",
            "tail call ptr @realloc(ptr %0, i64 %1)",
        ),
        (
            "define hidden void @typy_rt_heap_free(ptr %0)",
            "tail call void @free(ptr %0)",
        ),
    ] {
        assert!(ir.contains(hook), "{ir}");
        assert!(ir.contains(call), "{ir}");
    }
}

#[test]
fn the_heap_hooks_call_the_functions_the_project_names() {
    let (project, mut module) = compile("print(2.0 ** 0.5)\n");
    module.allocator = typy_tyir::TyirAllocator {
        allocate: "mi_malloc".to_owned(),
        reallocate: "mi_realloc".to_owned(),
        // A function the module defines with another type, which `typy.toml` refuses, still
        // leaves a module LLVM accepts.
        free: "__typy_main".to_owned(),
    };
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    assert!(ir.contains("tail call ptr @mi_malloc(i64 %0)"), "{ir}");
    assert!(
        ir.contains("tail call ptr @mi_realloc(ptr %0, i64 %1)"),
        "{ir}"
    );
    assert!(ir.contains("tail call void @__typy_main(ptr %0)"), "{ir}");
    assert!(!ir.contains("@malloc("), "{ir}");
}

/// Returns a module of one function, `test`, exported as a library with an `int`, a `float`
/// and a `bool` parameter and a `bool` result.
fn library_module() -> (Project, TyirModule) {
    let (project, mut module) =
        compile("def test(count: int, ratio: float, flag: bool) -> bool:\n    return flag\n");
    let function = module
        .functions
        .iter()
        .find(|function| function.name == "test")
        .unwrap()
        .id;
    module.library = Some(typy_tyir::TyirLibrary {
        name: "app".to_owned(),
        version: None,
        exports: vec![typy_tyir::TyirExport {
            name: "test".to_owned(),
            function,
            parameters: vec![
                typy_tyir::CType::Int64,
                typy_tyir::CType::Double,
                typy_tyir::CType::Bool,
            ],
            parameter_names: vec!["count".to_owned(), "ratio".to_owned(), "flag".to_owned()],
            result: Some(typy_tyir::CType::Bool),
        }],
    });
    (project, module)
}

#[test]
fn a_library_runs_its_module_when_it_loads_and_wraps_each_export() {
    let (project, module) = library_module();
    let ir = llvm_ir(
        &module,
        Target::host(),
        &CodegenOptions::new(Optimization::None),
        &project,
    )
    .unwrap();
    for line in [
        "@llvm.global_ctors = appending global [1 x { i32, ptr, ptr }] [{ i32, ptr, ptr } { i32 65535, ptr @typy.library.load, ptr null }]",
        "@llvm.global_dtors = appending global [1 x { i32, ptr, ptr }] [{ i32, ptr, ptr } { i32 65535, ptr @typy.library.unload, ptr null }]",
        "  call void @typy_rt_library_load(ptr @__typy_main)",
        "  call void @typy_rt_library_enter()",
        "  call void @typy_rt_library_leave()",
    ] {
        assert!(ir.lines().any(|each| each == line), "{line}\n{ir}");
    }
    assert!(
        ir.contains("call i64 @typy_rt_int_from_int64(i64 %0)"),
        "{ir}"
    );
    assert!(ir.contains("call void @typy_rt_int_decrement("), "{ir}");
    let header = typy_codegen::c_header(module.library.as_ref().unwrap());
    assert!(
        header.contains("bool test(int64_t count, double ratio, bool flag);"),
        "{header}"
    );
}

#[test]
fn an_export_takes_and_returns_a_bool_as_clang_lowers_it_on_each_target() {
    let (project, module) = library_module();
    for (triple, signature) in [
        (
            TargetTriple::X86_64LinuxGnu,
            "define zeroext i1 @test(i64 %0, double %1, i1 zeroext %2) {",
        ),
        (
            TargetTriple::X86_64LinuxMusl,
            "define zeroext i1 @test(i64 %0, double %1, i1 zeroext %2) {",
        ),
        (
            TargetTriple::Aarch64LinuxGnu,
            "define i1 @test(i64 %0, double %1, i1 %2) {",
        ),
    ] {
        let target = Target { triple };
        let ir = llvm_ir(
            &module,
            target,
            &CodegenOptions::new(Optimization::None),
            &project,
        )
        .unwrap();
        assert!(
            ir.lines().any(|each| each == signature),
            "{triple}: {signature}\n{ir}"
        );
    }
}
