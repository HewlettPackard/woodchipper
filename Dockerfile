FROM alpine:3.17.1 as kubectl

ARG KUBECTL_VERSION=v1.23.15

RUN apk add --no-cache curl && \
    curl -SsLf \
    "https://storage.googleapis.com/kubernetes-release/release/$KUBECTL_VERSION/bin/linux/amd64/kubectl" \
    -o /usr/local/bin/kubectl

FROM clux/muslrust:1.66.1-stable as builder

COPY src/ /app/src
COPY Cargo.toml Cargo.lock /app/

RUN cd /app && \
    cargo build --release --no-default-features

FROM scratch

COPY --from=builder \
    /app/target/x86_64-unknown-linux-musl/release/woodchipper \
    /usr/local/bin/woodchipper

COPY --from=kubectl \
    /usr/local/bin/kubectl \
    /usr/local/bin/kubectl

ENTRYPOINT ["/usr/local/bin/woodchipper"]
