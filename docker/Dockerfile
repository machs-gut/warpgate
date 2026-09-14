# syntax=docker/dockerfile:1.3-labs
# hadolint global ignore=DL3008
FROM rust:1.97.1-bullseye@sha256:02d78ca3f928195c2a907543de778adfd728ad7e2a24fdc6aef582b7c77842e0 AS build

ENV DEBIAN_FRONTEND=noninteractive
# Keep Bullseye security packages reproducible: the rolling security index can
# briefly reference packages already removed from the live pool. This snapshot
# matches the pinned 2026-08-03 Bullseye runtime base used below.
RUN sed -i -E 's#https?://[^ ]+/debian-security#https://snapshot.debian.org/archive/debian-security/20260803T000000Z#g' /etc/apt/sources.list
RUN printf 'Acquire::Check-Valid-Until "false";\n' > /etc/apt/apt.conf.d/99warpgate-snapshot

SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
    && apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates-java nodejs openjdk-17-jdk \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install --locked just@1.4.0

COPY . /opt/warpgate

# Needed to correctly embed the version number and the dirty state flag
COPY .git/ /opt/warpgate/.git/

# for rust-embed determinism
ENV SOURCE_DATE_EPOCH=0

WORKDIR /opt/warpgate

ARG CARGO_BUILD_JOBS=8
ARG CARGO_BUILD_PROFILE=release

# Keep frontend generation in its own cacheable layer so Rust build retries do
# not need to repeat npm/OpenAPI work.
RUN just npm ci \
    && just openapi \
    && just npm run build

RUN CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS}" \
    cargo build --features mysql,postgres,rdp-openssl-tls --profile "${CARGO_BUILD_PROFILE}" \
    && cp "target/${CARGO_BUILD_PROFILE}/warpgate" /opt/warpgate/warpgate-bin

FROM debian:bullseye-20260824@sha256:6f519a81440354a85eb592c5f32109ab80605f6b892455983a6f618bf87fabe9
LABEL maintainer=heywoodlh
ARG USER_ID=1000

ENV DEBIAN_FRONTEND=noninteractive
# Keep Bullseye security packages reproducible: the rolling security index can
# briefly reference packages already removed from the live pool. This snapshot
# matches the pinned 2026-08-03 Bullseye runtime base used below.
RUN sed -i -E 's#https?://[^ ]+/debian-security#https://snapshot.debian.org/archive/debian-security/20260803T000000Z#g' /etc/apt/sources.list
RUN printf 'Acquire::Check-Valid-Until "false";\n' > /etc/apt/apt.conf.d/99warpgate-snapshot
RUN <<EOF
  set -xe
  apt-get -y update -qq
  apt-get install --no-install-recommends --no-install-suggests -y \
    ca-certificates wget sqlite3
  apt-get clean
  rm -rf /var/lib/apt/lists/*
EOF

# RUN adduser warpgate --system --home /data
RUN adduser warpgate --disabled-password --gecos "" --uid ${USER_ID} --home /data --shell /usr/sbin/nologin

COPY --from=build /opt/warpgate/warpgate-bin /usr/local/bin/warpgate

VOLUME /data

ENV WARPGATE_CONFIG=/data/warpgate.yaml

HEALTHCHECK CMD warpgate healthcheck

ENV DOCKER=1
USER warpgate
ENTRYPOINT ["warpgate"]
CMD ["run"]
