#!/usr/bin/env bash
# Builds the runtime archive for each supported target with the `runtime`
# profile, then stages it where a compiler in <target>/debug or
# <target>/release finds it, <target>/lib/typy/<triple>/, as an installed
# compiler finds it in <prefix>/lib/typy/<triple>/. The copy under
# llvm-cov-target serves the compiler that `make coverage` builds there.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

target="${CARGO_TARGET_DIR:-target}"
for triple in x86_64-unknown-linux-gnu x86_64-unknown-linux-musl; do
    cargo rustc -p typy-runtime --profile runtime --locked --crate-type staticlib \
        --target "${triple}"
    for stage in "${target}" "${target}/llvm-cov-target"; do
        install -D -m 644 "${target}/${triple}/runtime/libtypy_runtime.a" \
            "${stage}/lib/typy/${triple}/libtypy_runtime.a"
    done
done

# The runtime instrumented with AddressSanitizer, for the conformance suite's
# sanitizer run (NFR-07). Rust's sanitizers need the pinned nightly toolchain;
# programs never link this archive.
triple=x86_64-unknown-linux-gnu
cargo +nightly-2026-11-05 rustc -p typy-runtime --profile runtime --locked \
    --crate-type staticlib --target "${triple}" --target-dir "${target}/asan" \
    -- -Zsanitizer=address
for stage in "${target}" "${target}/llvm-cov-target"; do
    install -D -m 644 "${target}/asan/${triple}/runtime/libtypy_runtime.a" \
        "${stage}/lib/typy-asan/${triple}/libtypy_runtime.a"
done
