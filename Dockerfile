# 1. Build Stage
FROM rust:1.87@sha256:251cec8da4689d180f124ef00024c2f83f79d9bf984e43c180a598119e326b84 as builder

WORKDIR /usr/src/RustyIP
COPY . .
RUN cargo build --release

# 2. Test Stage
FROM builder as tester
RUN cargo test --release

# 3. Distroless Stage
FROM gcr.io/distroless/cc-debian12@sha256:36598090a3f2c5f37d9d998d86f19b7b49ec1094c770119cbe219f2790489ebd
COPY --from=builder /usr/src/RustyIP/target/release/RustyIP /usr/local/bin/RustyIP

CMD ["RustyIP"]
