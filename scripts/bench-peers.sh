#!/usr/bin/env bash
# Installs the languages the cross-language benchmark compares TyPy with that
# the development image lacks: Go, the Temurin JDK, PyPy and Codon from their
# release archives, and mypyc, Cython and Nuitka into a CPython 3.14
# environment. Each archive is pinned to a version and the sha256 its project
# publishes, and is downloaded once into the bench-tools volume; a tool already
# in place is kept. CPython, clang, gcc, Rust and Node come with the image.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

root="${BENCH_TOOLS:-/var/cache/typy/bench}"
downloads="${root}/downloads"
mkdir -p "${downloads}"

# name, version, archive URL, sha256
archives=(
    "go 1.27.4 https://go.dev/dl/go1.27.4.linux-amd64.tar.gz 7226736c2339719cda28fbf3a54d71aa63bcefe376e98cef00034268dab24840"
    "jdk 25.0.5+8 https://github.com/adoptium/temurin25-binaries/releases/download/jdk-25.0.5%2B8/OpenJDK25U-jdk_x64_linux_hotspot_25.0.5_8.tar.gz 68982529869257d0ee6b824ec5aa270ff8b118bb438670da0b97ebdab178d7fb"
    "pypy 3.11-v7.3.23 https://downloads.python.org/pypy/pypy3.11-v7.3.23-linux64.tar.bz2 16f9f56e82d1f4ec95a324c1a8cacfd78afc7f0656c0a809a18725ef4391453a"
    "codon 0.20.6 https://github.com/exaloop/codon/releases/download/v0.20.6/codon-linux-x86_64.tar.gz 688772b279465e6d6ad7269741844f17dc5834eef4333f8a7851b6446f7143b6"
)

for entry in "${archives[@]}"; do
    read -r name version url sum <<<"${entry}"
    home="${root}/${name}-${version}"
    if [[ -d "${home}" ]]; then
        echo "${name} ${version}: in place"
        continue
    fi
    archive="${downloads}/${url##*/}"
    if [[ ! -f "${archive}" ]]; then
        echo "${name} ${version}: downloading ${url}"
        curl --fail --location --silent --show-error --output "${archive}.part" "${url}"
        mv "${archive}.part" "${archive}"
    fi
    echo "${sum}  ${archive}" | sha256sum --check --quiet
    # Each archive is unpacked into a directory of its own, as untrusted data.
    unpacked="$(mktemp -d "${root}/.unpack-XXXXXX")"
    tar --extract --file "${archive}" --directory "${unpacked}" --no-same-owner
    # Every archive holds one top directory, which becomes the tool's home.
    mv "${unpacked}"/* "${home}"
    rmdir "${unpacked}"
    echo "${name} ${version}: installed in ${home}"
done

environment="${root}/python-3.14"
if [[ ! -x "${environment}/bin/python" ]]; then
    uv venv --quiet --python /usr/local/bin/python3.14 "${environment}"
fi
VIRTUAL_ENV="${environment}" uv pip install --quiet --require-hashes \
    --requirement tests/bench/peers/requirements.txt
echo "mypyc, Cython and Nuitka: installed in ${environment}"
