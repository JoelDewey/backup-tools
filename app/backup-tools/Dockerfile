ARG RUST_VERSION=1.79.0
ARG ALPINE_VERSION=3.20

FROM rust:${RUST_VERSION}-alpine as build
WORKDIR /usr/src/myapp

COPY . .
RUN apk add --no-cache musl-dev \
    && cargo build --release


FROM alpine:${ALPINE_VERSION}

RUN apk add --no-cache rsync postgresql16-client tini tar mongodb-tools \
    && mkdir /destination \
    && mkdir /source \
    && mkdir /config

COPY --from=build --chmod=555 /usr/src/myapp/target/release/backup-tools /opt/

ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/opt/backup-tools"]
