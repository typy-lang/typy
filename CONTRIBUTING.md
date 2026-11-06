# Contributing to TyPy

Thank you for helping. This page says how a change gets from your working
copy into TyPy.

## Before you start

- Read the [development guide](docs/development/development-guide.md). It
  sets up the containers and lists every command.
- A change in behaviour starts in the
  [requirements specification](docs/requirements/srs.md). If your change
  needs a requirement that is not there, open an issue first.
- A design decision gets a decision record in `docs/adr/`.

## Making a change

1. Run `make hooks` once, so every commit runs the quality gate.
2. Make one change: the code, its tests and the documents it touches,
   together. A defect gets a failing test before the fix.
3. Run `make check`. It shall pass.
4. Commit with a subject in the imperative that says what the change does,
   such as "Add negative indexes to str". Name the requirement or decision
   in the body when the subject does not.
5. Open a pull request. The template asks what changes, why, and which
   requirement, issue or decision it relates to.

## Tests

Every requirement has an accepted and a rejected test, and an accepted
test is checked against CPython. The [test plan](docs/testing/test-plan.md)
says where each kind of test lives.

## Licence of contributions

Unless you state otherwise, a contribution you submit is licensed under
both the Apache License, Version 2.0 and the MIT license, as TyPy is.
