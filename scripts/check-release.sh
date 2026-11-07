#!/usr/bin/env bash
# Checks that the tag TAG names this tree's version, and that the changelog has
# a dated section for that version; writes the section to NOTES, the text of
# the release.
#
#   scripts/check-release.sh TAG NOTES
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

tag="${1:?usage: scripts/check-release.sh TAG NOTES}"
notes="${2:?usage: scripts/check-release.sh TAG NOTES}"
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
if [[ "${tag}" != "v${version}" ]]; then
    echo "check-release: the tag ${tag} does not match the version ${version}" >&2
    exit 1
fi
heading="$(grep -E "^## \[${version//./\\.}\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$" CHANGELOG.md || true)"
if [[ -z "${heading}" ]]; then
    echo "check-release: CHANGELOG.md has no section \"## [${version}] - YYYY-MM-DD\"" >&2
    exit 1
fi
# The section runs from its heading to the next one at the same level, or to
# the link definitions at the end of the changelog.
mkdir -p "$(dirname "${notes}")"
awk -v heading="${heading}" '
    $0 == heading { inside = 1; next }
    inside && (/^## / || /^\[[^]]+\]: /) { exit }
    inside { print }
' CHANGELOG.md | sed '/./,$!d' | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}' > "${notes}"
echo "check-release: ${tag} matches ${version}; notes in ${notes}"
