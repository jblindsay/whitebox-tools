ARG DOCKER_TAG=latest

FROM rust:latest AS builder

RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add x86_64-unknown-linux-musl

COPY . /wbt
WORKDIR /wbt
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest
COPY --from=builder /wbt/target/x86_64-unknown-linux-musl/release/whitebox_tools /usr/local/bin/

ENTRYPOINT ["whitebox_tools", "-v", "--wd=/data"]
