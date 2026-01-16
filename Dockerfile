FROM rust:alpine AS builder

# hadolint ignore=DL3018
RUN apk add --update --no-cache openssl-dev openssl-libs-static musl-dev pkgconfig clang lld curl

WORKDIR /var/lib/r2s-api-proxy

ADD Cargo.* .
ADD .cargo/ ./.cargo
ADD src/ ./src/

RUN --mount=type=cache,target=/var/lib/r2s-api-proxy/target cargo update && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cp /var/lib/r2s-api-proxy/target/x86_64-unknown-linux-musl/release/r2s-api-proxy /usr/local/bin/r2s-api-proxy

FROM alpine:3

# hadolint ignore=DL3018
RUN apk add --update --no-cache skopeo tini

COPY --from=builder /usr/local/bin/r2s-api-proxy /bin/r2s-api-proxy

ENTRYPOINT ["/sbin/tini", "--", "/bin/r2s-api-proxy", "--cache-dir", "/data"]
# CMD ["--endpoint", "https://ret.sh.cn/api"]