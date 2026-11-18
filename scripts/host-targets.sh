#!/usr/bin/env bash
# Prints the target triples this host builds for, one a line: the Linux
# targets on a Linux host, and macOS on a Mac, which only a Mac links for
# (ADR-0051, ADR-0055). The runtime build and the release archive read it.
set -euo pipefail

host="$(rustc -vV | sed -n 's/^host: //p')"
if [[ "${host}" == *-apple-darwin ]]; then
    echo aarch64-apple-darwin
else
    printf '%s\n' x86_64-unknown-linux-gnu x86_64-unknown-linux-musl aarch64-unknown-linux-gnu
fi
