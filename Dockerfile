# 1. Build Stage
FROM rust:1.87-slim as builder

# Set up build arguments for target architecture
ARG TARGETPLATFORM
ARG BUILDPLATFORM
ARG TARGETARCH

WORKDIR /usr/src/RustyIP
COPY . .

# Install cross-compilation dependencies and determine target
RUN apt-get update && apt-get install -y \
    gcc-multilib \
    libc6-dev-i386 \
    musl-tools \
    && rm -rf /var/lib/apt/lists/*

# Install cross-compilation toolchains based on architecture
RUN case "${TARGETARCH}" in \
    "arm64") \
        apt-get update && \
        apt-get install -y gcc-aarch64-linux-gnu libc6-dev-arm64-cross && \
        rm -rf /var/lib/apt/lists/* ;; \
    "arm") \
        apt-get update && \
        apt-get install -y gcc-arm-linux-gnueabihf libc6-dev-armhf-cross gcc-arm-linux-gnueabi libc6-dev-armel-cross && \
        rm -rf /var/lib/apt/lists/* ;; \
    *) \
        echo "No additional cross-compilation tools needed for ${TARGETARCH}" ;; \
    esac

# Add musl targets for static compilation based on architecture
RUN case "${TARGETARCH}" in \
    "amd64") \
        rustup target add x86_64-unknown-linux-musl && \
        echo "x86_64-unknown-linux-musl" > /tmp/rust_target ;; \
    "386") \
        rustup target add i686-unknown-linux-musl && \
        echo "i686-unknown-linux-musl" > /tmp/rust_target ;; \
    "arm64") \
        rustup target add aarch64-unknown-linux-musl && \
        echo "aarch64-unknown-linux-musl" > /tmp/rust_target ;; \
    "arm") \
        # Detect ARM variant from TARGETPLATFORM for proper target selection
        case "${TARGETPLATFORM}" in \
            *"v6"*) \
                rustup target add arm-unknown-linux-musleabi && \
                echo "arm-unknown-linux-musleabi" > /tmp/rust_target ;; \
            *"v7"*) \
                rustup target add armv7-unknown-linux-musleabihf && \
                echo "armv7-unknown-linux-musleabihf" > /tmp/rust_target ;; \
            *) \
                rustup target add armv7-unknown-linux-musleabihf && \
                echo "armv7-unknown-linux-musleabihf" > /tmp/rust_target ;; \
        esac ;; \
    *) \
        echo "Unsupported architecture: ${TARGETARCH}" && exit 1 ;; \
    esac

# Build static binary for the target architecture with maximum size optimization
RUN RUST_TARGET=$(cat /tmp/rust_target) && \
    echo "Building ultra-optimized binary for ${RUST_TARGET}..." && \
    echo "Target: ${RUST_TARGET}" && \
    echo "Platform: ${TARGETPLATFORM}" && \
    RUSTFLAGS="-C target-cpu=generic -C strip=symbols -C panic=abort" \
    cargo build \
        --profile release \
        --target ${RUST_TARGET} \
        --features small-binary \
        --no-default-features && \
    echo "Build completed successfully!" && \
    echo "Binary size:" && \
    ls -lh ./target/${RUST_TARGET}/release/RustyIP && \
    echo "Binary info:" && \
    file ./target/${RUST_TARGET}/release/RustyIP || true

# 2. Test Stage
FROM builder as tester

# Install required packages for testing
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    openssl \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy test scripts
COPY test/e2e-integration.py /usr/src/RustyIP/
COPY test/generate-test-certs.sh /usr/src/RustyIP/

# Run Rust tests with size optimizations
RUN RUST_TARGET=$(cat /tmp/rust_target) && \
    echo "Running tests for ${RUST_TARGET}..." && \
    cargo test \
        --profile release \
        --target ${RUST_TARGET} \
        --features small-binary \
        --no-default-features

# Run integration tests with HTTPS server
RUN chmod +x generate-test-certs.sh&& ./generate-test-certs.sh&& python3 e2e-integration.py

# 3. Distroless Stage
FROM gcr.io/distroless/static-debian12@sha256:5c7e2b465ac6a2a4e5d4bad46165e4f6c4d3b71fe7bb267d3c73e38095cf2e65
ARG TARGETARCH
ARG TARGETPLATFORM
RUN case "${TARGETARCH}" in \
    "amd64") echo "x86_64-unknown-linux-musl" > /tmp/rust_target ;; \
    "386") echo "i686-unknown-linux-musl" > /tmp/rust_target ;; \
    "arm64") echo "aarch64-unknown-linux-musl" > /tmp/rust_target ;; \
    "arm") \
        case "${TARGETPLATFORM}" in \
            *"v6"*) echo "arm-unknown-linux-musleabi" > /tmp/rust_target ;; \
            *"v7"*) echo "armv7-unknown-linux-musleabihf" > /tmp/rust_target ;; \
            *) echo "armv7-unknown-linux-musleabihf" > /tmp/rust_target ;; \
        esac ;; \
    esac
COPY --from=builder /usr/src/RustyIP/target/$(cat /tmp/rust_target)/release/RustyIP /usr/local/bin/

CMD ["RustyIP"]
