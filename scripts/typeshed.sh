#!/usr/bin/env bash
# Fetches the standard library stubs of the typeshed revision that
# crates/typy-stdlib/typeshed-revision pins, about 3 MB, into TYPESHED_CACHE,
# /var/cache/typy/typeshed in the dev container, once; the signature test of
# the standard library modules TyPy gives reads them (ADR-0097, ADR-0119).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
revision="$(tr -d '[:space:]' < crates/typy-stdlib/typeshed-revision)"
cache="${TYPESHED_CACHE:-/var/cache/typy/typeshed}"
target="${cache}/${revision}"
if [ -d "${target}/stdlib" ]; then
    exit 0
fi
mkdir -p "${target}"
archive="$(mktemp)"
trap 'rm -f "${archive}"' EXIT
curl --fail --silent --show-error --location --output "${archive}" \
    "https://codeload.github.com/python/typeshed/tar.gz/${revision}"
tar --extract --gzip --file "${archive}" --directory "${target}" \
    --strip-components=1 "typeshed-${revision}/stdlib"
echo "typeshed ${revision}: stdlib stubs in ${target}"
