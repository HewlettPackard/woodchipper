FROM alpine:3.9 as kubectl

ARG KUBECTL_VERSION=v1.14.2

RUN apk add --no-cache curl && \
    curl -SsLf \
      "https://storage.googleapis.com/kubernetes-release/release/$KUBECTL_VERSION/bin/linux/amd64/kubectl" \
      -o /usr/local/bin/kubectl

FROM clux/muslrust:stable-2019-04-24 as builder

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
