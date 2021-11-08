FROM debian:buster-slim as runner

RUN apt update; apt install -y libssl1.1 libopus-dev ffmpeg

FROM rust:1.55.0 as builder

WORKDIR /usr/src

RUN rustup update nightly
RUN rustup default nightly
RUN rustup target add x86_64-unknown-linux-musl

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/target \
    cargo build --release --out-dir . -Z unstable-options

FROM runner

COPY --from=builder /usr/src/marine-tts ./

USER root

CMD ["./marine-tts"]