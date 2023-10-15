ARG RUST_VERSION=1.73
ARG ALPINE_VERSION=3.18
ARG GOSU_VERSION=1.16

FROM rust:${RUST_VERSION}-alpine as build
WORKDIR /usr/src/myapp

COPY . .
RUN apk add --no-cache musl-dev \
    && cargo build --release


FROM alpine:${ALPINE_VERSION}

RUN apk add --no-cache rsync postgresql-client tini \
    && addgroup --system backuptools \
    && adduser --system --no-create-home backuptools -G backuptools \
    && mkdir /destination \
    && chown backuptools:backuptools /destination \
    && mkdir /source \
    && chown backuptools:backuptools /source \
    && mkdir /config \
    && chown backuptools:backuptools /config

COPY --from=build --chown=backuptools:backuptools --chmod=555 /usr/src/myapp/target/release/backup-tools /opt/

USER backuptools:backuptools

ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/opt/backup-tools"]
