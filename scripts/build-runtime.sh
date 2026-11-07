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
