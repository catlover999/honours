# 1. Build stages for filter_dp. Using cargo-chef for optimal layering
FROM docker.io/rust:latest AS builder 
RUN rustup target add wasm32-wasi
WORKDIR /filter_dp

COPY filter_dp/Cargo.toml filter_dp/Cargo.lock ./
COPY filter_dp/src src
RUN cargo build --release --target wasm32-wasi
RUN du target/wasm32-wasi/release/filter_dp.wasm 

# 2. Fluent Bit Stage
#FROM cr.fluentbit.io/fluent/fluent-bit:2.2.2-debug as fluent-bit
#WORKDIR /fluent-bit
FROM debian:bullseye-slim as fluent-builder
ENV DEBIAN_FRONTEND noninteractive
# Install dependencies
RUN echo "deb http://deb.debian.org/debian bullseye-backports main" >> /etc/apt/sources.list && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    ca-certificates \
    cmake \
    git \
    make \
    tar \
    libssl-dev \
    libsasl2-dev \
    pkg-config \
    libsystemd-dev/bullseye-backports \
    zlib1g-dev \
    libpq-dev \
    postgresql-server-dev-all \
    flex \
    bison \
    libyaml-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /fluent-bit/bin /fluent-bit/etc /fluent-bit/log
COPY fluent-bit/ /src/fluent-bit
WORKDIR /src/fluent-bit/build/
COPY ./fluent-bit /src/fluent-bit
RUN cmake -DFLB_RELEASE=On \
    -DFLB_JEMALLOC=On \
    -DFLB_TLS=On \
    -DFLB_SHARED_LIB=Off \
    -DFLB_EXAMPLES=Off \
    -DFLB_HTTP_SERVER=On \
    -DFLB_IN_EXEC=Off \
    -DFLB_IN_SYSTEMD=On \
    -DFLB_OUT_KAFKA=On \
    -DFLB_OUT_PGSQL=On \
    -DFLB_LOG_NO_CONTROL_CHARS=On \
    -DFLB_CHUNK_TRACE=On \
    .. && \
    make -j "$(getconf _NPROCESSORS_ONLN)" && \
    install bin/fluent-bit /fluent-bit/bin/
COPY fluent-bit/conf/*.conf /fluent-bit/etc/

FROM fluent-builder as fluent-runner
COPY --from=builder /filter_dp/target/wasm32-wasi/release/filter_dp.wasm .
COPY fluent-bit.conf .
COPY input input
RUN mkdir output
RUN bin/fluent-bit -c fluent-bit.conf

# 3. Evaualtion stage
FROM docker.io/jupyter/minimal-notebook:latest as notebook
WORKDIR /notebook
COPY project.ipynb requirements.txt /notebook/
RUN pip install -r requirements.txt
COPY input input/
COPY --from=fluent-runner /fluent-bit/output/ output/


