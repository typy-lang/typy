#!/usr/bin/env bash
# Builds the release archive of the compiler for the host, x86_64 or AArch64
# Linux or macOS on Apple silicon, in dist/: the `typy` executable and a
# runtime archive for each target the host builds for, every Linux target on
# a Linux host and macOS on a Mac (ADR-0051, ADR-0055), in the layout of an
# installation prefix (architecture section 9), with the licences, the
# third-party notices and licences, README and changelog. The tarball is
# reproducible: its entries are sorted, owned by root and dated from the last
# commit, which GNU tar writes, `gtar` on a Mac. The unpacked archive must
# then build and run a program, which proves it works away from the build
# tree.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

target="${CARGO_TARGET_DIR:-target}"
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
host="$(rustc -vV | sed -n 's/^host: //p')"
name="typy-${version}-${host}"
stage="dist/${name}"

scripts/build-runtime.sh
cargo build -p typy-cli --release --locked
rm -rf "${stage}" "dist/${name}.tar.gz"
mkdir -p "${stage}/bin"
install -m 755 "${target}/release/typy" "${stage}/bin/typy"
scripts/check-executable-needs.sh "${stage}/bin/typy"
for triple in $(scripts/host-targets.sh); do
    mkdir -p "${stage}/lib/typy/${triple}"
    install -m 644 "${target}/lib/typy/${triple}/libtypy_runtime.a" \
        "${stage}/lib/typy/${triple}/libtypy_runtime.a"
done
install -m 644 LICENSE-APACHE LICENSE-MIT THIRD-PARTY-NOTICES.md README.md CHANGELOG.md \
    "${stage}/"

# The licences of the code the archive holds that is not TyPy's: the crates
# and the LLVM that `typy` links, and Rust's standard library, which `typy`
# links and whose core every program gets through the runtime archive.
licenses="${stage}/licenses"
install -d "${licenses}"
# cargo metadata reads every package of Cargo.lock, including those of other
# platforms that a Linux build never downloads, so they are fetched first and
# the listing itself stays offline.
cargo fetch --locked
cargo about generate --frozen -c about.toml -m crates/typy-cli/Cargo.toml about.hbs \
    -o "${licenses}/crates.html"
install -m 644 "${LLVM_SYS_231_PREFIX}/LICENSE.TXT" "${licenses}/llvm.txt"
install -m 644 "$(rustc --print sysroot)/share/doc/rust/COPYRIGHT-library.html" \
    "${licenses}/rust-library.html"

epoch="$(git log -1 --format=%ct)"
gnu_tar=tar
if [[ "$(uname -s)" == Darwin ]]; then
    gnu_tar=gtar
fi
"${gnu_tar}" --sort=name --owner=0 --group=0 --numeric-owner --mtime="@${epoch}" \
    -C dist -cf - "${name}" | gzip -n -9 > "dist/${name}.tar.gz"
if command -v sha256sum > /dev/null; then
    (cd dist && sha256sum "${name}.tar.gz" > "${name}.tar.gz.sha256")
else
    (cd dist && shasum -a 256 "${name}.tar.gz" > "${name}.tar.gz.sha256")
fi

check="$(mktemp -d)"
trap 'rm -rf "${check}"' EXIT
tar -xzf "dist/${name}.tar.gz" -C "${check}"
printf 'print("hello from", 1 + 2)\n' > "${check}/hello.py"
output="$(TYPY_HOME="${check}/home" "${check}/${name}/bin/typy" run "${check}/hello.py")"
if [[ "${output}" != "hello from 3" ]]; then
    echo "package-release: the unpacked compiler printed ${output@Q}" >&2
    exit 1
fi
echo "package-release: dist/${name}.tar.gz"
