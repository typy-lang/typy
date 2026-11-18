#!/usr/bin/env bash
# Builds LLVM from its source archive for the host it runs on and packs the
# libraries, the headers and llvm-config that llvm-sys links TyPy's compiler
# with into OUTPUT/llvm-VERSION-HOST.tar.zst, with its SHA-256 beside it
# (ADR-0054). Only the AArch64 and x86 back ends are built, and no optional
# library such as zlib, zstd, libxml2 or z3, so the build depends on the
# host's C and C++ libraries alone. llvm.yml runs it on each host TyPy runs on,
# after checking the archive's attestation.
#
# Usage: scripts/build-llvm.sh VERSION HOST OUTPUT
# The archive llvm-project-VERSION.src.tar.xz must be in the current directory.
set -euo pipefail

version="$1"
host="$2"
output="$3"

if [[ ! "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "build-llvm: '${version}' is not an LLVM release such as 23.1.6" >&2
    exit 2
fi
options=()
case "${host}" in
    x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu) ;;
    aarch64-apple-darwin)
        # macOS 11 is the oldest macOS on Apple silicon (ADR-0055).
        options+=(-DCMAKE_OSX_DEPLOYMENT_TARGET=11.0 -DCMAKE_OSX_ARCHITECTURES=arm64)
        ;;
    *)
        echo "build-llvm: '${host}' is not a host TyPy runs on" >&2
        exit 2
        ;;
esac

source="llvm-project-${version}.src"
name="llvm-${version}-${host}"
prefix="${PWD}/${name}"
tar -xJf "${source}.tar.xz"
rm "${source}.tar.xz"

cmake -G Ninja -S "${source}/llvm" -B build \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="${prefix}" \
    -DLLVM_TARGETS_TO_BUILD="AArch64;X86" \
    -DLLVM_ENABLE_ASSERTIONS=OFF \
    -DLLVM_ENABLE_BINDINGS=OFF \
    -DLLVM_ENABLE_FFI=OFF \
    -DLLVM_ENABLE_LIBEDIT=OFF \
    -DLLVM_ENABLE_LIBPFM=OFF \
    -DLLVM_ENABLE_LIBXML2=OFF \
    -DLLVM_ENABLE_Z3_SOLVER=OFF \
    -DLLVM_ENABLE_ZLIB=OFF \
    -DLLVM_ENABLE_ZSTD=OFF \
    -DLLVM_INCLUDE_BENCHMARKS=OFF \
    -DLLVM_INCLUDE_DOCS=OFF \
    -DLLVM_INCLUDE_EXAMPLES=OFF \
    -DLLVM_INCLUDE_TESTS=OFF \
    -DLLVM_DISTRIBUTION_COMPONENTS="llvm-libraries;llvm-headers;llvm-config" \
    "${options[@]}"
# The distribution builds only its components and the tools that generate
# their sources, not every LLVM tool.
ninja -C build install-distribution
# LLVM's licence travels with its binaries (Apache-2.0 WITH LLVM-exception).
cp "${source}/llvm/LICENSE.TXT" "${prefix}/"
"${prefix}/bin/llvm-config" --version
"${prefix}/bin/llvm-config" --system-libs

mkdir -p "${output}"
tar -cf - "${name}" | zstd -19 -T0 -o "${output}/${name}.tar.zst"
cd "${output}"
if command -v sha256sum > /dev/null; then
    sha256sum "${name}.tar.zst" > "${name}.tar.zst.sha256"
else
    shasum -a 256 "${name}.tar.zst" > "${name}.tar.zst.sha256"
fi
cat "${name}.tar.zst.sha256"
