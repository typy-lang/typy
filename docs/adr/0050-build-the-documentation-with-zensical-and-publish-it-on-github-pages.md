# ADR-0050: Build the documentation with Zensical and publish it on GitHub Pages

## Status

Accepted

## Date

2026-11-17

## Context

TyPy's documentation is 81 Markdown pages under `docs/` in release 0.5.0:
the user documentation and the engineering documents. They are read on
GitHub, where a reader finds no navigation, no search and no page for the
release they run. DOC-05 asks for a site at <https://typy-lang.org>, built
in strict mode from the sources of each release. The pages stay what the
checks already read: markdownlint lints them, and the documentation test
judges every Python sample in them (DOC-04) and reads the troubleshooting
page that `typy explain` embeds (DOC-03).

The state of the Markdown site generators, read from their projects on
2026-11-17:

| Tool | State |
| --- | --- |
| MkDocs 1.6.1 | Unmaintained, and closed to outside contributions since 2026-10-02 |
| MkDocs 2.0 | A pre-release rewrite with no plugins, and no migration path that Material for MkDocs supports |
| Material for MkDocs 9.8.1 | In maintenance mode, with its end of life on 2027-05-05 |
| Zensical 0.1 | A stable line since 2026-11-05, from the team behind Material for MkDocs; it builds an existing `mkdocs.yml` project, with a subset of its plugins |

The sources are the announcement of Zensical
(<https://squidfunk.github.io/mkdocs-material/blog/2025/11/05/zensical/>),
the releases of Material for MkDocs
(<https://github.com/squidfunk/mkdocs-material/releases>), its notes on
MkDocs 2.0
(<https://squidfunk.github.io/mkdocs-material/blog/2026/02/18/mkdocs-2.0/>),
and the discussion of MkDocs' maintenance
(<https://github.com/orgs/ProperDocs/discussions/33>).

Zensical 0.1.1, tried on these pages on 2026-11-17, builds all of them in
about two seconds. Its strict mode fails on a link to a page or an anchor
that does not exist, and on a link that leaves `docs/`. It does not report
a page that the navigation leaves out, or a navigation entry whose page
does not exist, and it ignores the `validation.nav` settings that MkDocs
reads for both.

## Options considered

1. **MkDocs 1.6.1 with Material for MkDocs 9.8.** The site is built today,
   with tools that receive no fixes and that reach their end of life within
   six months.
2. **Zensical, from `mkdocs.yml`.** The pages stay Markdown and the
   configuration stays the one Material for MkDocs reads, so a return to
   Material is a change of the command that builds the site.
3. **Sphinx.** It is mature and links to the Python documentation through
   intersphinx. All 81 pages, markdownlint and the sample test of DOC-04
   are Markdown, so the pages would be converted or read through a
   Markdown parser of Sphinx's, and its strength, documentation generated
   from Python source, gives nothing to a compiler written in Rust.

## Decision

TyPy takes option 2.

- **Tool.** Zensical 0.1.1 with every dependency is pinned by version and
  sha256 in `scripts/docs-requirements.txt`. `scripts/build-docs.sh`
  installs it into a CPython 3.14 environment once and runs
  `zensical build --strict --clean` into `site/`. `make docs` runs the
  script in the dev container and `make check` runs `make docs`, so a
  broken link fails the quality gate.
- **Configuration.** `mkdocs.yml` at the repository root holds only
  settings that Material for MkDocs 9.8 also reads. Icons come from the
  sets both ship, and the system fonts replace web fonts, so a page loads
  nothing from another host to show its text. The palette is the theme's
  deep purple, with an accent of TyPy's own in `docs/stylesheets/extra.css`,
  since the theme's accent gives the current page in the navigation less
  than the 4.5:1 contrast WCAG 2.2 level AA asks of text.
- **Navigation.** The `nav` of `mkdocs.yml` names every page under
  `docs/`, the decision records included, and the documentation test fails
  on a page it leaves out or a page it names that does not exist, which
  Zensical does not check.
- **Layout.** The user documentation takes the sections of Python's
  documentation, in its order: What's new, Tutorial, Library reference,
  Language reference, Setup and usage, Extending and embedding, then
  Diagnostics, which is TyPy's own. Section names and order are all that
  is taken from the Python documentation; where TyPy behaves as CPython
  does, a page links to <https://docs.python.org> rather than quoting it.
  The engineering documents keep their directories, and the changelog page
  includes `CHANGELOG.md` through `pymdownx.snippets`.
- **Publishing.** The workflow `.github/workflows/pages.yml` builds the
  site with `scripts/build-docs.sh` on the runner, with read-only
  permissions and without the dev image, and deploys it from a job that
  alone may write to Pages, in the `github-pages` environment.
  `release.yml` calls it once it has created a release, since a release
  created with the workflow token starts no `release` event; a release
  published by hand starts it too, and it runs by hand on a release tag.
  The custom domain is a setting of the repository.

## Consequences

- A reader gets navigation, search, light and dark modes and one URL per
  page, for the release they run.
- A broken link or anchor, or a page missing from the navigation, fails
  the quality gate, so the site stays whole between releases.
- Each decision record adds a line to the navigation in `mkdocs.yml`.
- Zensical is young: a 0.1 line that may change its configuration. The pin
  keeps a build reproducible, and an upgrade is a change of the
  requirements file that `make docs` checks.
- The site shows the documents of the last published release; changes on
  `main` wait for the next release.

## Related requirements

DOC-01 to DOC-05.
