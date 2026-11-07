#!/usr/bin/env bash
# Measures line and branch coverage of the compiler and runtime with the
# pinned nightly toolchain, which branch coverage needs, and fails below 95
# percent of either (NFR-11). The conformance harness is test code and is left
# out, as in `make coverage`. The build has a target directory of its own:
# profiles from the stable build of `make coverage` in the same directory would
# not match and spoil the report.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

target="${CARGO_TARGET_DIR:-target}"
coverage="${target}/coverage-branches"
# The tests find the runtime archives beside the compiler they build.
mkdir -p "${coverage}"
# cargo-llvm-cov cleans only a directory that carries the cache directory tag.
printf 'Signature: 8a477f597d28d172789f06886806bc55\n' > "${coverage}/CACHEDIR.TAG"
rm -rf "${coverage}/lib"
cp -r "${target}/lib" "${coverage}/lib"
report="$(CARGO_LLVM_COV_TARGET_DIR="${coverage}" cargo +nightly-2026-11-05 llvm-cov nextest \
    --workspace --locked --branch --status-level fail --final-status-level fail \
    --summary-only --color never --ignore-filename-regex 'typy-conformance/')"
printf '%s\n' "${report}"
# cargo-llvm-cov has no branch threshold, so the TOTAL row is read: its tenth
# and thirteenth columns are the line and branch percentages.
printf '%s\n' "${report}" | awk '
    $1 == "TOTAL" {
        lines = $10 + 0; branches = $13 + 0; found = 1
        printf "coverage-branches: lines %.2f%%, branches %.2f%%, of 95%% each\n", lines, branches
        if (lines < 95 || branches < 95) exit 1
    }
    END { if (!found) exit 1 }'
