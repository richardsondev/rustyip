# 1. Build Stage
FROM rust:1.87@sha256:251cec8da4689d180f124ef00024c2f83f79d9bf984e43c180a598119e326b84 as builder

WORKDIR /usr/src/RustyIP
COPY . .

# Add musl target for static compilation
RUN rustup target add x86_64-unknown-linux-musl

# Build static binary
RUN cargo build --release --target x86_64-unknown-linux-musl

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
COPY --from=builder /usr/src/RustyIP/target/x86_64-unknown-linux-musl/release/RustyIP /usr/local/bin/RustyIP

CMD ["RustyIP"]
