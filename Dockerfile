FROM rust:1.48.0 as builder

WORKDIR /usr/src

RUN rustup target add x86_64-unknown-linux-musl

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/target \
    cargo install --path .


FROM runner
COPY --from=builder /usr/local/cargo/bin/marine-tts .
USER 1000

CMD ["./marine-tts"]