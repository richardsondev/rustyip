# 1. Build Stage
FROM rust:1.87@sha256:251cec8da4689d180f124ef00024c2f83f79d9bf984e43c180a598119e326b84 as builder

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

# Set up cross-compilation environment
RUN RUST_TARGET=$(cat /tmp/rust_target) && \
    case "${RUST_TARGET}" in \
    "i686-unknown-linux-musl") \
        echo "[target.i686-unknown-linux-musl]" >> ~/.cargo/config.toml && \
        echo "linker = \"gcc\"" >> ~/.cargo/config.toml ;; \
    "aarch64-unknown-linux-musl") \
        echo "[target.aarch64-unknown-linux-musl]" >> ~/.cargo/config.toml && \
        echo "linker = \"aarch64-linux-gnu-gcc\"" >> ~/.cargo/config.toml ;; \
    "arm-unknown-linux-musleabi") \
        echo "[target.arm-unknown-linux-musleabi]" >> ~/.cargo/config.toml && \
        echo "linker = \"arm-linux-gnueabi-gcc\"" >> ~/.cargo/config.toml ;; \
    "armv7-unknown-linux-musleabihf") \
        echo "[target.armv7-unknown-linux-musleabihf]" >> ~/.cargo/config.toml && \
        echo "linker = \"arm-linux-gnueabihf-gcc\"" >> ~/.cargo/config.toml ;; \
    esac

# Build static binary for the target architecture
RUN RUST_TARGET=$(cat /tmp/rust_target) && \
    cargo build --release --target ${RUST_TARGET}

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

# Run Rust tests
RUN cargo test --release

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
