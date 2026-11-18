#!/usr/bin/env bash
# Fails when the Linux executable EXECUTABLE needs more than the documents
# promise: glibc 2.39 or later and libstdc++, which Ubuntu 24.04 and Debian 13
# have, and no other shared library. LLVM is linked statically and built with
# no optional library (ADR-0054), so a new dependency is a change to review.
#
# Usage: scripts/check-executable-needs.sh EXECUTABLE
set -euo pipefail

executable="$1"
newest="GLIBC_2.39"
allowed='^(libc\.so\.6|libm\.so\.6|libgcc_s\.so\.1|libstdc\+\+\.so\.6|ld-linux-(x86-64|aarch64)\.so\.[12])$'

status=0
while read -r library; do
    if [[ ! "${library}" =~ ${allowed} ]]; then
        echo "check-executable-needs: ${executable} needs ${library}" >&2
        status=1
    fi
done < <(objdump -p "${executable}" | awk '$1 == "NEEDED" { print $2 }')

glibc="$(objdump -T "${executable}" | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)"
if [[ "$(printf '%s\n' "${newest}" "${glibc}" | sort -V | tail -1)" != "${newest}" ]]; then
    echo "check-executable-needs: ${executable} needs ${glibc}, newer than ${newest}" >&2
    status=1
fi
exit "${status}"
