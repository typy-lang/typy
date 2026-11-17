#!/usr/bin/env bash
# Builds the documentation site from mkdocs.yml into site/ with Zensical in
# strict mode, so a broken link or anchor fails the build (DOC-05, ADR-0050).
# Zensical and its dependencies are installed from
# scripts/docs-requirements.txt, each file checked against its sha256, into
# a CPython 3.14 environment under DOCS_TOOLS, /var/cache/typy/docs in the
# dev container; the environment is made once, kept in step with the file,
# and made again in a container built anew. `make docs` runs this script in the dev container, and the Pages
# workflow runs it on the runner.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

environment="${DOCS_TOOLS:-/var/cache/typy/docs}/python-3.14"
if [[ ! -x "${environment}/bin/python" ]]; then
    uv venv --quiet --python 3.14 "${environment}"
fi
VIRTUAL_ENV="${environment}" uv pip sync --quiet --require-hashes \
    scripts/docs-requirements.txt

# A clean build renders every page again, so a warning that a cached page
# gave once is given on every run.
NO_COLOR=1 "${environment}/bin/zensical" build --strict --clean
"${environment}/bin/python" scripts/check-docs-nav.py
