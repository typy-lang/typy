#!/usr/bin/env bash
# Fails when a requirement of the specification is defined twice or has no row
# in the test plan's traceability matrix, or when a document cites a requirement, constraint,
# assumption, use case, issue or decision ID that is not defined.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

readonly SRS=docs/requirements/srs.md
readonly PLAN=docs/testing/test-plan.md
readonly ID='[A-Z]{1,3}-[0-9]{2}'
status=0

defined_requirements() {
    grep -oE "^\| ${ID} \|" "${SRS}" | tr -d '| ' | grep -vE '^(OI|UC)-' | sort -u
}
matrix_rows() {
    sed -n '/^## 8. Traceability matrix/,$p' "${PLAN}" |
        grep -oE "^\| ${ID} \|" | tr -d '| ' | sort -u
}

# An ID is never reused, so each is defined once.
duplicated="$(grep -oE "^\| ${ID} \|" "${SRS}" | tr -d '| ' | sort | uniq -d)"
if [[ -n "${duplicated}" ]]; then
    echo "check-traceability: IDs defined more than once in ${SRS}:" >&2
    echo "${duplicated}" >&2
    status=1
fi

missing="$(comm -23 <(defined_requirements | grep -vE '^A-') <(matrix_rows) |
    grep -vE '^C-01$' || true)"
if [[ -n "${missing}" ]]; then
    echo "check-traceability: requirements without a row in ${PLAN}:" >&2
    echo "${missing}" >&2
    status=1
fi

defined="$( { defined_requirements; grep -oE "^\| OI-[0-9]{2} \|" "${SRS}" | tr -d '| ';
    grep -oE '\bUC-[0-9]{2}\b' docs/modeling/use-case-model.md;
    ls docs/adr | grep -oE '^[0-9]{4}' | sed 's/^/ADR-/'; } | sort -u)"
# The pattern ends at a word boundary, so the specification pack's
# three-digit IDs, such as LEX-001, are not taken for two-digit ones.
cited="$(grep -rhoE "\b(${ID}|ADR-[0-9]{4})\b" docs --include='*.md' | sort -u)"
unknown="$(comm -23 <(echo "${cited}") <(echo "${defined}") | grep -vE '^R-[0-9]{2}$' || true)"
if [[ -n "${unknown}" ]]; then
    echo "check-traceability: IDs cited but not defined:" >&2
    echo "${unknown}" >&2
    status=1
fi
exit "${status}"
