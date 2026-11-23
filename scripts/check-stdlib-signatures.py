#!/usr/bin/env python3
"""Fails when a standard library module TyPy gives differs from typeshed's stub of it.

Each module of crates/typy-stdlib/lib is typed Python. This script reads it and the stub
of the module of the same name at the typeshed revision crates/typy-stdlib/typeshed-revision
pins, which scripts/typeshed.sh fetches, and compares their public names and, for each
function, its parameters across the stub's overloads: their names, how each is passed and
whether it may be left out, and its result where the stub writes a plain type. A difference
TyPy keeps on purpose is listed in crates/typy-stdlib/signature-differences.toml with its
reason; any other difference, and a listed one that no longer exists, fails (ADR-0097,
ADR-0119).
"""

from __future__ import annotations

import ast
import os
import pathlib
import sys
import tomllib
from dataclasses import dataclass

ROOT = pathlib.Path(__file__).resolve().parent.parent
STDLIB = ROOT / "crates" / "typy-stdlib"
REVISION = (STDLIB / "typeshed-revision").read_text().strip()
STUBS = pathlib.Path(os.environ.get("TYPESHED_CACHE", "/var/cache/typy/typeshed")) / REVISION
FLOOR = (3, 12)

POSITIONAL = "positional"
EITHER = "positional or keyword"
KEYWORD = "keyword"
STAR = "*args"
STARSTAR = "**kwargs"


@dataclass(frozen=True)
class Parameter:
    """One parameter of a function: its name, how it is passed and whether it may be left out."""

    name: str
    kind: str
    optional: bool


@dataclass
class Function:
    """The signatures of one function: the module's one, or each of the stub's overloads."""

    signatures: list[list[Parameter]]
    results: list[str]


def parameters(function: ast.FunctionDef) -> list[Parameter]:
    """Returns the parameters of `function`."""
    arguments = function.args
    found: list[Parameter] = []
    positional = arguments.posonlyargs + arguments.args
    defaults = [None] * (len(positional) - len(arguments.defaults)) + list(arguments.defaults)
    for index, argument in enumerate(positional):
        kind = POSITIONAL if index < len(arguments.posonlyargs) else EITHER
        # typeshed writes a positional-only parameter of an older stub as `__name`.
        if argument.arg.startswith("__") and not argument.arg.endswith("__"):
            kind = POSITIONAL
        found.append(Parameter(argument.arg.lstrip("_") or argument.arg, kind, defaults[index] is not None))
    if arguments.vararg is not None:
        found.append(Parameter(arguments.vararg.arg, STAR, True))
    for argument, default in zip(arguments.kwonlyargs, arguments.kw_defaults, strict=True):
        found.append(Parameter(argument.arg, KEYWORD, default is not None))
    if arguments.kwarg is not None:
        found.append(Parameter(arguments.kwarg.arg, STARSTAR, True))
    return found


def available(test: ast.expr) -> bool:
    """Returns whether a `sys.version_info` test holds at TyPy's Python floor, 3.12."""
    if (
        isinstance(test, ast.Compare)
        and isinstance(test.left, ast.Attribute)
        and test.left.attr == "version_info"
        and len(test.ops) == 1
        and isinstance(test.comparators[0], ast.Tuple)
    ):
        version = tuple(
            element.value
            for element in test.comparators[0].elts
            if isinstance(element, ast.Constant) and isinstance(element.value, int)
        )
        operator = test.ops[0]
        if isinstance(operator, ast.GtE):
            return FLOOR >= version
        if isinstance(operator, ast.Lt):
            return FLOOR < version
    return True


def collect(statements: list[ast.stmt], module: dict[str, Function | str], stub_dir: pathlib.Path | None) -> None:
    """Adds the public names `statements` define to `module`, following `from X import *`."""
    for statement in statements:
        if isinstance(statement, ast.If):
            collect(statement.body if available(statement.test) else statement.orelse, module, stub_dir)
        elif isinstance(statement, ast.FunctionDef):
            entry = module.get(statement.name)
            function = entry if isinstance(entry, Function) else Function([], [])
            function.signatures.append(parameters(statement))
            function.results.append(ast.unparse(statement.returns) if statement.returns else "None")
            module[statement.name] = function
        elif isinstance(statement, ast.ClassDef):
            module[statement.name] = "class"
        elif isinstance(statement, ast.Assign) and len(statement.targets) == 1:
            target = statement.targets[0]
            if isinstance(target, ast.Name):
                value = statement.value
                if isinstance(value, ast.Name) and isinstance(module.get(value.id), Function):
                    module[target.id] = module[value.id]
                elif not (isinstance(value, ast.Call) and ast.unparse(value.func) in {"TypeVar", "ParamSpec"}):
                    module[target.id] = "value"
        elif isinstance(statement, ast.AnnAssign) and isinstance(statement.target, ast.Name):
            module[statement.target.id] = "value"
        elif (
            isinstance(statement, ast.ImportFrom)
            and stub_dir is not None
            and statement.module is not None
            and any(alias.name == "*" for alias in statement.names)
        ):
            other = stub_dir / f"{statement.module}.pyi"
            if other.is_file():
                collect(ast.parse(other.read_text()).body, module, stub_dir)


def public(module: dict[str, Function | str]) -> dict[str, Function | str]:
    """Returns the names of `module` a program may use: those without a leading underscore."""
    return {name: entry for name, entry in module.items() if not name.startswith("_")}


def plain(result: str) -> bool:
    """Returns whether a stub's result names no type variable or protocol of typeshed."""
    return not any(part in result for part in ("_T", "Supports", "Self", "_KT", "_VT", "_S", "_P"))


def compare(name: str, own: Function, stub: Function, allowed: set[str]) -> list[str]:
    """Returns the differences of the function `name` from its stub, but for the parameters
    `allowed` lists."""
    problems: list[str] = []
    mine = {parameter.name: parameter for parameter in own.signatures[0]}
    theirs: dict[str, set[str]] = {}
    optional: dict[str, bool] = {}
    for signature in stub.signatures:
        for parameter in signature:
            theirs.setdefault(parameter.name, set()).add(parameter.kind)
            optional[parameter.name] = optional.get(parameter.name, False) or parameter.optional
    for parameter_name, kinds in theirs.items():
        if parameter_name in allowed:
            continue
        parameter = mine.get(parameter_name)
        if parameter is None:
            problems.append(f"{name}: no parameter `{parameter_name}`")
            continue
        if parameter.kind not in kinds and not (parameter.kind == EITHER and kinds <= {EITHER, KEYWORD}):
            problems.append(f"{name}: `{parameter_name}` is passed {parameter.kind}, not {' or '.join(sorted(kinds))}")
        if parameter.optional != optional[parameter_name]:
            problems.append(f"{name}: `{parameter_name}` may{'' if parameter.optional else ' not'} be left out")
    for parameter_name in mine:
        if parameter_name not in theirs:
            problems.append(f"{name}: a parameter `{parameter_name}` the stub has not")
    results = set(stub.results)
    if len(results) == 1 and plain(next(iter(results))) and own.results[0] not in results:
        problems.append(f"{name}: gives {own.results[0]}, not {next(iter(results))}")
    return problems


def main() -> int:
    if not (STUBS / "stdlib").is_dir():
        print(f"typeshed {REVISION} is not in {STUBS}; run `make typeshed`", file=sys.stderr)
        return 1
    differences = tomllib.loads((STDLIB / "signature-differences.toml").read_text()).get("difference", [])
    used: set[int] = set()
    problems: list[str] = []
    for source in sorted((STDLIB / "lib").glob("*.py")):
        module_name = source.stem
        stub_path = STUBS / "stdlib" / f"{module_name}.pyi"
        own: dict[str, Function | str] = {}
        collect(ast.parse(source.read_text()).body, own, None)
        stub: dict[str, Function | str] = {}
        collect(ast.parse(stub_path.read_text()).body, stub, STUBS / "stdlib")
        own, stub = public(own), public(stub)
        for name, entry in stub.items():
            if name not in own:
                problems.append(f"{module_name}: no name `{name}`")
            elif isinstance(entry, Function) and isinstance(own[name], Function):
                allowed: set[str] = set()
                own_parameters = {parameter.name for parameter in own[name].signatures[0]}  # type: ignore[union-attr]
                for index, difference in enumerate(differences):
                    if difference["module"] == module_name and name in difference["names"]:
                        allowed.add(difference["parameter"])
                        # A listed parameter the module now has is a difference no longer kept.
                        if difference["parameter"] not in own_parameters:
                            used.add(index)
                found = compare(name, own[name], entry, allowed)  # type: ignore[arg-type]
                problems.extend(f"{module_name}.{problem}" for problem in found)
        for name in own:
            if name not in stub:
                problems.append(f"{module_name}: a name `{name}` the stub has not")
    for index, difference in enumerate(differences):
        if index not in used:
            problems.append(f"signature-differences.toml lists a difference that no longer exists: {difference}")
    for problem in problems:
        print(problem, file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
