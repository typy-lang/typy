#!/usr/bin/env bash
# Runs each fuzz target for FUZZ_SECONDS seconds (60 by default), starting
# from the conformance programs. An input that runs longer than 10 seconds is
# a hang. A crash leaves the input under fuzz/artifacts/ and fails the run.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

toolchain="+nightly-2026-11-05"
seconds="${FUZZ_SECONDS:-60}"
corpus="fuzz/corpus/stages"
mkdir -p "${corpus}"
find tests/conformance -name '*.py' -exec cp {} "${corpus}/" \;
cd fuzz
# The prebuilt cargo-fuzz is a musl binary and would default to its own target,
# where the sanitizer cannot work, so the target is named.
cargo "${toolchain}" fuzz run --target x86_64-unknown-linux-gnu stages "../${corpus}" -- \
    -max_total_time="${seconds}" -timeout=10 -max_len=4096
