#!/usr/bin/env bash
# Prints `light=true` for $GITHUB_OUTPUT when every file a pull request
# changes is text that no compiled test reads, and `light=false` otherwise.
# A light run checks the documents without building the dev image. A Markdown
# page outside the engineering directories counts only while it holds no
# Python block, since the documentation test compiles each (DOC-04); the
# troubleshooting page never counts, since `typy explain` embeds it.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

engineering='^docs/(adr|architecture|design|development|diagrams|modeling|requirements|research|testing)/'

# Returns success when a pull request may change `path` and still run light.
light_path() {
    local path="$1"
    case "${path}" in
        docs/diagnostics/troubleshooting.md) return 1 ;;
        docs/*.md)
            [[ "${path}" =~ ${engineering} || ! -f "${path}" ]] \
                || ! grep -q '^```python' "${path}"
            ;;
        docs/diagrams/* | docs/stylesheets/* | mkdocs.yml) return 0 ;;
        scripts/docs-requirements.txt | scripts/check-docs-nav.py) return 0 ;;
        .github/*.md | .github/ISSUE_TEMPLATE/*) return 0 ;;
        */*) return 1 ;;
        *.md) return 0 ;;
        *) return 1 ;;
    esac
}

light=true
while IFS= read -r path; do
    if ! light_path "${path}"; then
        echo "ci-changes: ${path} needs the full run" >&2
        light=false
        break
    fi
done < <(git diff --name-only "$1" HEAD)
echo "light=${light}"
