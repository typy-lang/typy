#!/usr/bin/env bash
# Runs the runtime's unit tests under Miri, which reports undefined behaviour:
# out-of-bounds and misaligned accesses, use after free, and reads of
# uninitialised memory (ADR-0006, NFR-07). An `int` word that is not inline is
# an address, so the runtime uses exposed provenance and Miri is told so. The
# unit tests create objects without releasing them; leaks are checked on whole
# programs by the conformance suite's sanitizer run instead (NFR-08).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

MIRIFLAGS="-Zmiri-permissive-provenance -Zmiri-ignore-leaks" \
    cargo +nightly-2026-11-05 miri test -p typy-runtime --locked
