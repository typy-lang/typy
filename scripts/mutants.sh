#!/usr/bin/env bash
# Runs cargo-mutants (test plan, mutation testing) in a container limited to 6 GB of memory
# with no swap above it, 4 CPUs, 1024 processes and no network. A mutant can make a loop
# allocate without end; the limit stops it inside the container, and the machine keeps its
# memory. The container runs an image of the dev container as it is now, so it has the tools
# installed there without a download; the image is removed afterwards.
#
#   scripts/mutants.sh diff [BASE]    the lines changed since BASE, by default the work not
#                                     yet committed, new files included; minutes per change
#   scripts/mutants.sh package NAME   every mutant of one package; hours for a large one
#
# Each mutant is tested with its own package's tests. A crate whose tests need the runtime
# archives built beside the compiler (typy-build, typy-driver, typy-cli) has its mutants
# reported as failing their baseline, so its changes are left to the end-to-end tests.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

usage="usage: scripts/mutants.sh diff [BASE] | package NAME"
mode="${1:?${usage}}"
image="typy-lang-mutants:local"
mkdir -p .tmp/mutants

case "${mode}" in
    diff)
        base="${2:-HEAD}"
        output=".tmp/mutants/diff"
        changes=".tmp/mutants/changes.diff"
        git diff "${base}" -- 'crates/*.rs' > "${changes}"
        # New files are not in `git diff` until they are added; each is a diff of its own.
        git ls-files --others --exclude-standard -- 'crates/*.rs' | while read -r file; do
            git diff --no-index /dev/null "${file}" || true
        done >> "${changes}"
        if ! grep -q '^+++ ' "${changes}"; then
            echo "mutants: no Rust source changed since ${base}"
            exit 0
        fi
        arguments=(--workspace --in-diff "${changes}" --jobs 2 --output "${output}"
            --exclude 'crates/typy-build/**' --exclude 'crates/typy-driver/**'
            --exclude 'crates/typy-cli/**')
        ;;
    package)
        package="${2:?${usage}}"
        arguments=(--package "${package}" --jobs 2 --no-shuffle
            --output ".tmp/mutants/${package}")
        ;;
    *)
        echo "${usage}" >&2
        exit 2
        ;;
esac

dev="$(docker compose -f .devcontainer/compose.yaml ps -q dev)"
if [ -z "${dev}" ]; then
    echo "mutants: the dev container is not running; run make dev" >&2
    exit 1
fi
# The copy would keep the dev container's Compose labels, and Compose would then take its
# container for one of the project's and stop it; these labels make it no Compose container.
docker commit --pause=false \
    --change "LABEL com.docker.compose.project=typy-lang-mutants" \
    --change "LABEL com.docker.compose.service=mutants" \
    --change "LABEL com.docker.compose.oneoff=True" \
    "${dev}" "${image}" >/dev/null
trap 'docker rmi "${image}" >/dev/null' EXIT
docker run --rm --init --network none \
    --memory 6g --memory-swap 6g --cpus 4 --pids-limit 1024 \
    --user "$(id -u):$(id -g)" \
    --volume "${PWD}:/workspaces/typy-lang" \
    --volume typy-lang_cargo-registry:/usr/local/cargo/registry \
    --volume typy-lang_cargo-git:/usr/local/cargo/git \
    --workdir /workspaces/typy-lang \
    --env CARGO_NET_OFFLINE=true \
    --env CARGO_TERM_COLOR=never \
    "${image}" \
    env -u CARGO_TARGET_DIR cargo mutants "${arguments[@]}"
