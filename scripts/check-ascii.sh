#!/usr/bin/env bash
# Fails when a tracked text file holds a byte outside ASCII. Rendered SVG files
# are left out: PlantUML writes UML stereotypes with the characters << and >>
# as single guillemet characters, which is standard UML notation.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

status=0
while IFS= read -r -d '' path; do
    if LC_ALL=C grep -nP '[^\x00-\x7F]' "${path}" >/dev/null; then
        LC_ALL=C grep -nP '[^\x00-\x7F]' "${path}" | sed "s|^|${path}:|" >&2
        status=1
    fi
done < <(git ls-files -z --cached --others --exclude-standard \
    -- ':!*.svg' ':!*.png' ':!Cargo.lock')
exit "${status}"
