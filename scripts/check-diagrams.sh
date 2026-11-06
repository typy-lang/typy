#!/usr/bin/env bash
# Fails when a rendered SVG differs from the one in the git index, or when a
# diagram source in the index has no SVG beside it. Run it after rendering, so
# the working tree holds fresh SVG files; it also works before a commit,
# because it compares with the index and not with the last commit.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

status=0
if ! git diff --exit-code --stat -- docs/diagrams; then
    echo "check-diagrams: rendered SVG files differ from the index; stage them" >&2
    status=1
fi

while IFS= read -r source; do
    [[ "$(basename "${source}")" == "style.puml" ]] && continue
    svg="${source%.puml}.svg"
    if ! git ls-files --error-unmatch -- "${svg}" >/dev/null 2>&1; then
        echo "check-diagrams: ${source} is staged but ${svg} is not" >&2
        status=1
    fi
done < <(git ls-files -- 'docs/diagrams/*.puml')
exit "${status}"
