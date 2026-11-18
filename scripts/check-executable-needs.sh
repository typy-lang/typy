#!/usr/bin/env bash
# Fails when the executable EXECUTABLE needs more than the documents promise.
# On Linux: glibc 2.39 or later and libstdc++, which Ubuntu 24.04 and
# Debian 13 have, and no other shared library. On macOS: macOS 11 or later
# and the system's libSystem, libc++ and libiconv alone, which every macOS
# has; Rust's standard library links libiconv on Apple's systems (ADR-0055).
# LLVM is linked statically and built with no optional library (ADR-0054),
# so a new dependency is a change to review.
#
# Usage: scripts/check-executable-needs.sh EXECUTABLE
set -euo pipefail

executable="$1"
status=0

# Reports each of `libraries`, one a line, that `allowed` does not match.
check_libraries() {
    local allowed="$1" libraries="$2"
    while read -r library; do
        if [[ -n "${library}" && ! "${library}" =~ ${allowed} ]]; then
            echo "check-executable-needs: ${executable} needs ${library}" >&2
            status=1
        fi
    done <<< "${libraries}"
}

if [[ "$(uname -s)" == Darwin ]]; then
    check_libraries '^/usr/lib/(libSystem\.B|libc\+\+\.1|libiconv\.2)\.dylib$' \
        "$(otool -L "${executable}" | awk 'NR > 1 { print $1 }')"
    minimum="$(otool -l "${executable}" | awk '$1 == "minos" { print $2; exit }')"
    if [[ "${minimum}" != 11.0 ]]; then
        echo "check-executable-needs: ${executable} needs macOS ${minimum}, not 11.0" >&2
        status=1
    fi
    exit "${status}"
fi

newest="GLIBC_2.39"
check_libraries '^(libc\.so\.6|libm\.so\.6|libgcc_s\.so\.1|libstdc\+\+\.so\.6|ld-linux-(x86-64|aarch64)\.so\.[12])$' \
    "$(objdump -p "${executable}" | awk '$1 == "NEEDED" { print $2 }')"
glibc="$(objdump -T "${executable}" | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)"
if [[ "$(printf '%s\n' "${newest}" "${glibc}" | sort -V | tail -1)" != "${newest}" ]]; then
    echo "check-executable-needs: ${executable} needs ${glibc}, newer than ${newest}" >&2
    status=1
fi
exit "${status}"
