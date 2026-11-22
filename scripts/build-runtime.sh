#!/usr/bin/env bash
# Builds the runtime archives for each target the host builds for: the Linux
# targets on a Linux host and macOS on a Mac (ADR-0051, ADR-0055). The
# `runtime` profile, optimised for speed, gives the archive debug and
# `--release` builds link, and `runtime-small`, optimised for size, the one
# `--release-small` builds link (ADR-0100). Each is staged where a compiler in
# <target>/debug or <target>/release finds it, <target>/lib/typy/<triple>/ and
# <target>/lib/typy-small/<triple>/, as an installed compiler finds them in
# <prefix>/lib/. The copy under llvm-cov-target serves the compiler that
# `make coverage` builds there.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

target="${CARGO_TARGET_DIR:-target}"
host="$(rustc -vV | sed -n 's/^host: //p')"

# Copies the archive `built` to lib/typy`suffix`/`triple`/ in each staging
# directory, with the tools of both systems.
stage() {
    local built="$1" suffix="$2" triple="$3"
    for directory in "${target}" "${target}/llvm-cov-target"; do
        mkdir -p "${directory}/lib/typy${suffix}/${triple}"
        install -m 644 "${built}" "${directory}/lib/typy${suffix}/${triple}/libtypy_runtime.a"
    done
}

for triple in $(scripts/host-targets.sh); do
    cargo rustc -p typy-runtime --profile runtime --locked --crate-type staticlib \
        --target "${triple}"
    stage "${target}/${triple}/runtime/libtypy_runtime.a" "" "${triple}"
    cargo rustc -p typy-runtime --profile runtime-small --locked --crate-type staticlib \
        --target "${triple}"
    stage "${target}/${triple}/runtime-small/libtypy_runtime.a" "-small" "${triple}"
done

# The runtime instrumented with AddressSanitizer, for the conformance suite's
# sanitizer run on Linux (NFR-07); macOS on Apple silicon has no leak
# detection, which the run needs. Rust's sanitizers need the pinned nightly
# toolchain; programs never link this archive. It is built for the host, whose
# programs the sanitizer run builds and runs.
if [[ "${host}" == *-linux-* ]]; then
    cargo +nightly-2026-11-05 rustc -p typy-runtime --profile runtime --locked \
        --crate-type staticlib --target "${host}" --target-dir "${target}/asan" \
        -- -Zsanitizer=address
    stage "${target}/asan/${host}/runtime/libtypy_runtime.a" "-asan" "${host}"
fi
