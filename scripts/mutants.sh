#!/usr/bin/env bash
# Runs cargo-mutants on one package (test plan, mutation testing) in a container limited to
# 6 GB of memory with no swap above it, 4 CPUs, 1024 processes and no network. A mutant can
# make a loop allocate without end; the limit stops it inside the container, and the
# machine keeps its memory. The container runs an image of the dev container as it is now,
# so it has the tools installed there without a download; the image is removed afterwards.
# Each package's tests run on their own, so a crate whose tests need the runtime archives
# built beside the compiler (typy-build, typy-driver, typy-cli) is not a package for it.
#
# With `suites` as the second argument, only the mutants an earlier run of the package did not
# catch are tested again, against the conformance suite and the driver and command-line
# tests, which run whole programs. The runtime archives the dev container built are copied to
# `target/lib/` first, and from there into each copy of the tree, where the tests find them
# beside the compiler they build; so this mode suits a compiler package, not `typy-runtime`.
# The sanitizer run of the suite is left out, since it repeats the suite for the runtime.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

package="${1:?usage: scripts/mutants.sh PACKAGE [suites]}"
mode="${2:-own}"
image="typy-lang-mutants:local"
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
mkdir -p .tmp/mutants
arguments=(--package "${package}" --jobs 2 --no-shuffle --output ".tmp/mutants/${package}")
if [ "${mode}" = "suites" ]; then
    rm -rf target/lib
    mkdir -p target
    docker cp "${dev}:/var/cache/typy/target/lib" target/lib
    arguments+=(--iterate --copy-target true
        --test-package "${package}" --test-package typy-conformance
        --test-package typy-driver --test-package typy-cli
        --cargo-test-arg=-E --cargo-test-arg="not binary(sanitized)")
fi
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
