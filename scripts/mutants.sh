#!/usr/bin/env bash
# Runs cargo-mutants on one package (test plan, mutation testing) in a container limited to
# 6 GB of memory with no swap above it, 4 CPUs, 1024 processes and no network. A mutant can
# make a loop allocate without end; the limit stops it inside the container, and the
# machine keeps its memory. The container runs an image of the dev container as it is now,
# so it has the tools installed there without a download; the image is removed afterwards.
# Each package's tests run on their own, so a crate whose tests need the runtime archives
# built beside the compiler (typy-build, typy-driver, typy-cli) is not a package for it.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

package="${1:?usage: scripts/mutants.sh PACKAGE}"
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
    env -u CARGO_TARGET_DIR cargo mutants --package "${package}" --jobs 2 --no-shuffle \
    --output ".tmp/mutants/${package}"
