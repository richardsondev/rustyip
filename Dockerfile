# 1. Build Stage
FROM rust:1.87@sha256:25038aa450210c53cf05dbf7b256e1df1ee650a58bb46cbc7d6fa79c1d98d083 as builder

ARG TARGETPLATFORM

# Setup build environment based on TARGETPLATFORM
# This RUN command determines the RUST_TARGET_TRIPLE and installs necessary tools and Rust targets.
# It also writes the RUST_TARGET_TRIPLE to /build_env.sh to be used by subsequent RUN commands.
RUN apt-get update && \
    apt-get install -y --no-install-recommends musl-tools ca-certificates && \
    RUST_TARGET_TRIPLE="" && \
    case "${TARGETPLATFORM}" in \
        "linux/amd64") RUST_TARGET_TRIPLE='x86_64-unknown-linux-musl' ;; \
        "linux/arm64") RUST_TARGET_TRIPLE='aarch64-unknown-linux-musl' ;; \
        "linux/arm/v7") RUST_TARGET_TRIPLE='armv7-unknown-linux-musleabihf' ;; \
        *) echo "Warning: Unsupported TARGETPLATFORM: ${TARGETPLATFORM}. Defaulting to x86_64-unknown-linux-musl." >&2 ; \
           RUST_TARGET_TRIPLE='x86_64-unknown-linux-musl' ;; \
    esac && \
    echo "Building for RUST_TARGET_TRIPLE=${RUST_TARGET_TRIPLE} on TARGETPLATFORM=${TARGETPLATFORM}" && \
    rustup target add "${RUST_TARGET_TRIPLE}" && \
    echo "export RUST_TARGET_TRIPLE=${RUST_TARGET_TRIPLE}" > /build_env.sh && \
    echo "Successfully prepared build environment for ${RUST_TARGET_TRIPLE}" && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/RustyIP
COPY . .

# Build the application using the RUST_TARGET_TRIPLE from /build_env.sh
RUN . /build_env.sh && \
    echo "Building with RUST_TARGET_TRIPLE=${RUST_TARGET_TRIPLE}" && \
    cargo build --target "${RUST_TARGET_TRIPLE}" --release && \
    mkdir -p /app && \
    cp "target/${RUST_TARGET_TRIPLE}/release/RustyIP" /app/RustyIP && \
    echo "Successfully built RustyIP for ${RUST_TARGET_TRIPLE}"

# 2. Test Stage
FROM builder as tester
# TARGETPLATFORM ARG is implicitly available if the build stage had it.
# WORKDIR is inherited from the builder stage if not re-specified, but good to be explicit.
WORKDIR /usr/src/RustyIP

# Test the application using the RUST_TARGET_TRIPLE from /build_env.sh (created in builder stage)
RUN . /build_env.sh && \
    echo "Testing with RUST_TARGET_TRIPLE=${RUST_TARGET_TRIPLE} (from /build_env.sh: $(cat /build_env.sh))" && \
    cargo test --target "${RUST_TARGET_TRIPLE}" --release && \
    echo "Successfully tested RustyIP for ${RUST_TARGET_TRIPLE}"

# 3. Distroless Stage
FROM gcr.io/distroless/static-debian12@sha256:633d5fa264a127052ca34c3fdaf81ef5a58204770736df9047745919a5b318f6
COPY --from=builder /app/RustyIP /usr/local/bin/RustyIP

CMD ["RustyIP"]
