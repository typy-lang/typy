# Development image for typy-lang: the pinned Rust toolchain, native build
# dependencies, debuggers and the cargo tools used by the project tasks.

ARG RUST_VERSION=1.99.1
FROM rust:${RUST_VERSION}-slim-trixie

ARG USERNAME=dev
ARG USER_UID=1000
ARG USER_GID=1000

# Versions are pinned so every rebuild produces the same tool set.
ARG CARGO_BINSTALL_VERSION=1.26.2
ARG CARGO_NEXTEST_VERSION=0.9.151
ARG CARGO_LLVM_COV_VERSION=0.9.1
ARG CARGO_DENY_VERSION=0.20.2
ARG CARGO_AUDIT_VERSION=0.22.2
ARG CARGO_WATCH_VERSION=8.5.3
ARG CARGO_INSTA_VERSION=1.49.0
ARG CARGO_EXPAND_VERSION=1.0.127
ARG CARGO_MACHETE_VERSION=0.9.2
ARG JUST_VERSION=1.59.0
ARG HYPERFINE_VERSION=2.0.0

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        clang \
        cmake \
        curl \
        gdb \
        git \
        less \
        libssl-dev \
        lld \
        lldb \
        mold \
        openssh-client \
        pkg-config \
        procps \
    && rm -rf /var/lib/apt/lists/*

# Login shells reset PATH from /etc/profile, so cargo's bin directory is added
# back there for them.
RUN printf 'export PATH="%s/bin:$PATH"\n' "${CARGO_HOME}" > /etc/profile.d/cargo.sh

RUN groupadd --gid "${USER_GID}" "${USERNAME}" \
    && useradd --uid "${USER_UID}" --gid "${USER_GID}" --create-home \
        --shell /bin/bash "${USERNAME}" \
    && mkdir -p /var/cache/typy/target "/home/${USERNAME}/.history" \
    && chown -R "${USERNAME}:${USERNAME}" \
        "${CARGO_HOME}" "${RUSTUP_HOME}" /var/cache/typy "/home/${USERNAME}/.history"

USER ${USERNAME}

# cargo-binstall is built from crates.io source, which cargo verifies by
# checksum. It then fetches prebuilt binaries for the other tools, which keeps
# the image build to minutes instead of compiling each tool.
RUN cargo install --locked cargo-binstall --version "${CARGO_BINSTALL_VERSION}" \
    && cargo binstall --no-confirm --locked \
        "cargo-nextest@${CARGO_NEXTEST_VERSION}" \
        "cargo-llvm-cov@${CARGO_LLVM_COV_VERSION}" \
        "cargo-deny@${CARGO_DENY_VERSION}" \
        "cargo-audit@${CARGO_AUDIT_VERSION}" \
        "cargo-watch@${CARGO_WATCH_VERSION}" \
        "cargo-insta@${CARGO_INSTA_VERSION}" \
        "cargo-expand@${CARGO_EXPAND_VERSION}" \
        "cargo-machete@${CARGO_MACHETE_VERSION}" \
        "just@${JUST_VERSION}" \
        "hyperfine@${HYPERFINE_VERSION}" \
    && rm -rf "${CARGO_HOME}/registry" "${CARGO_HOME}/git" \
    && mkdir -p "${CARGO_HOME}/registry" "${CARGO_HOME}/git"

# The toolchain file in the repository decides the components; installing them
# here means the first cargo command in the container does not download them.
COPY --chown=${USERNAME}:${USERNAME} rust-toolchain.toml /tmp/toolchain/
RUN cd /tmp/toolchain && rustup toolchain install && rm -rf /tmp/toolchain

# The documentation checks run in this container too. Node comes from the
# official release, checked against its published SHA-256, and
# markdownlint-cli2 is pinned so every machine applies the same rules.
ARG NODE_VERSION=26.13.0
ARG NODE_SHA256_X64=d28a96af6c38596f6d9d97c0ee531c13d238d757742387d1928de822d959e997
ARG NODE_SHA256_ARM64=406fe3fbde1c054adff8b297c16fdb5af6659db327c1ac8acd46b56ef1af9a92
ARG MARKDOWNLINT_CLI2_VERSION=0.24.0
USER root
RUN case "$(dpkg --print-architecture)" in \
        amd64) arch=x64; sum="${NODE_SHA256_X64}" ;; \
        arm64) arch=arm64; sum="${NODE_SHA256_ARM64}" ;; \
        *) echo "unsupported architecture" >&2; exit 1 ;; \
    esac \
    && file="node-v${NODE_VERSION}-linux-${arch}.tar.xz" \
    && curl -fsSL -o "/tmp/${file}" "https://nodejs.org/dist/v${NODE_VERSION}/${file}" \
    && echo "${sum}  /tmp/${file}" | sha256sum --check --strict \
    && tar -xJf "/tmp/${file}" -C /usr/local --strip-components=1 \
        --exclude='*/README.md' --exclude='*/CHANGELOG.md' --exclude='*/LICENSE' \
    && rm "/tmp/${file}" \
    && npm install --global --no-audit --no-fund \
        "markdownlint-cli2@${MARKDOWNLINT_CLI2_VERSION}" \
    && npm cache clean --force

# LLVM 23 for typy-codegen (ADR-0005) comes from apt.llvm.org, because Debian
# ships only LLVM 19. The signing key is accepted only with the expected
# fingerprint. musl-tools provides the static Linux target.
ARG LLVM_MAJOR=23
ARG LLVM_KEY_FINGERPRINT=6084F3CF814B57C1CF12EFD515CF4D18AF4F7421
RUN apt-get update \
    && apt-get install -y --no-install-recommends gnupg \
    && curl -fsSL -o /tmp/llvm.asc https://apt.llvm.org/llvm-snapshot.gpg.key \
    && fingerprint="$(gpg --show-keys --with-colons /tmp/llvm.asc \
        | awk -F: '/^fpr:/ { print $10; exit }')" \
    && test "${fingerprint}" = "${LLVM_KEY_FINGERPRINT}" \
    && mkdir -p /etc/apt/keyrings \
    && gpg --dearmor -o /etc/apt/keyrings/apt.llvm.org.gpg /tmp/llvm.asc \
    && echo "deb [signed-by=/etc/apt/keyrings/apt.llvm.org.gpg] https://apt.llvm.org/trixie/ llvm-toolchain-trixie-${LLVM_MAJOR} main" \
        > /etc/apt/sources.list.d/apt.llvm.org.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        "llvm-${LLVM_MAJOR}-dev" \
        "libpolly-${LLVM_MAJOR}-dev" \
        libffi-dev \
        libxml2-dev \
        libzstd-dev \
        musl-tools \
        zlib1g-dev \
    && apt-get purge -y gnupg \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/* /tmp/llvm.asc

ENV LLVM_SYS_231_PREFIX=/usr/lib/llvm-${LLVM_MAJOR}

# The CPython oracles (ADR-0008) are exact builds installed with uv, so the
# conformance suite can run every version from inside this container.
ARG UV_VERSION=0.13.7
ARG UV_SHA256_X64=c5221ff41b7892bfc223543883600125f6a47fe845093e0f5bdff4f359196d83
ARG UV_SHA256_ARM64=71279f8d3c67241e807b9d8db872d1bf541579fb9f08b7d1eb1682196d1e57dc
ARG ORACLE_PYTHONS="3.12.16 3.13.18 3.14.10 3.15.2"
RUN case "$(dpkg --print-architecture)" in \
        amd64) arch=x86_64; sum="${UV_SHA256_X64}" ;; \
        arm64) arch=aarch64; sum="${UV_SHA256_ARM64}" ;; \
        *) echo "unsupported architecture" >&2; exit 1 ;; \
    esac \
    && file="uv-${arch}-unknown-linux-gnu.tar.gz" \
    && curl -fsSL -o "/tmp/${file}" \
        "https://github.com/astral-sh/uv/releases/download/${UV_VERSION}/${file}" \
    && echo "${sum}  /tmp/${file}" | sha256sum --check --strict \
    && tar -xzf "/tmp/${file}" -C /usr/local/bin --strip-components=1 \
    && rm "/tmp/${file}" \
    && UV_PYTHON_INSTALL_DIR=/opt/python UV_PYTHON_BIN_DIR=/usr/local/bin \
        uv python install ${ORACLE_PYTHONS} \
    && chmod -R a+rX /opt/python
USER ${USERNAME}

# Fuzzing (NFR-09), Miri and the instrumented sanitizers (OI-16) need one pinned
# nightly toolchain; stable stays the toolchain of every build. This layer is
# last, so adding it leaves the cached layers above it unchanged.
ARG NIGHTLY=nightly-2026-11-05
ARG CARGO_FUZZ_VERSION=0.13.2
RUN rustup toolchain install "${NIGHTLY}" --profile minimal --component miri,rust-src \
    && cargo binstall --no-confirm --locked "cargo-fuzz@${CARGO_FUZZ_VERSION}" \
    && rm -rf "${CARGO_HOME}/registry" "${CARGO_HOME}/git" \
    && mkdir -p "${CARGO_HOME}/registry" "${CARGO_HOME}/git"

ENV CARGO_TARGET_DIR=/var/cache/typy/target

WORKDIR /workspaces/typy-lang
