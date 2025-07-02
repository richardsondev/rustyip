# 1. Build Stage
FROM rust:1.88 as builder

WORKDIR /usr/src/RustyIP
COPY . .
RUN cargo build --release

# 2. Test Stage
FROM builder as tester
RUN cargo test --release

# 3. Distroless Stage
FROM gcr.io/distroless/cc-debian11
COPY --from=builder /usr/src/RustyIP/target/release/RustyIP /usr/local/bin/RustyIP

CMD ["RustyIP"]
