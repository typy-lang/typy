# About these documents

These documents describe TyPy: its language, its library and its tools,
and the engineering documents behind them. They are written in Markdown in
the `docs/` directory of the
[TyPy repository](https://github.com/typy-lang/typy), and each release
publishes the documents of its own sources at <https://typy-lang.org>.

## How the site is built

[Zensical](https://zensical.org/) builds the site from `mkdocs.yml` at the
root of the repository. The build runs in strict mode, so a link to a page
or an anchor that does not exist stops it, and the test suite checks that
every page is in the navigation (DOC-05). `make docs` builds the site into
`site/`, as the
[development guide](../development/development-guide.md) describes.

## How the samples are checked

Every Python sample in the user documentation is a test. The test suite
compiles each sample with the current compiler: an accepted sample must
run and print what CPython prints, and a rejected one must give the
diagnostic code at the place its page states (DOC-04). The
[troubleshooting](../diagnostics/troubleshooting.md) page has a section
for every diagnostic code (DOC-03), and `typy explain` prints the same
text.

Where TyPy behaves as CPython does, these pages link to the
[Python documentation](https://docs.python.org/3/) rather than repeat it.

## Reporting a problem

Report a mistake in a page, a sample that does not do what its page says,
or a broken link as an issue at <https://github.com/typy-lang/typy/issues>.
Report a security problem privately, as the
[security policy](https://github.com/typy-lang/typy/blob/main/SECURITY.md)
describes.

## Contributing

A change to these documents goes through a pull request, like any other
change to TyPy.
[CONTRIBUTING.md](https://github.com/typy-lang/typy/blob/main/CONTRIBUTING.md)
describes the steps, and the
[development guide](../development/development-guide.md) lists the
commands.

## Credits

The site is built with Zensical, copyright Zensical and contributors,
which is distributed under the MIT licence. Its theme includes icons from
Lucide, Material Design Icons and Font Awesome Free, each under the
licence its project gives.
