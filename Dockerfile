# 1. Build stages for filter_dp
FROM docker.io/rust:latest AS builder 
RUN rustup target add wasm32-wasi
WORKDIR /filter_dp

COPY filter_dp/Cargo.toml filter_dp/Cargo.lock ./
COPY filter_dp/src src
RUN cargo build --release --target wasm32-wasi

# 2. Fluent Bit Stage
FROM debian:bookworm-slim as fluent-builder

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    libcurl4-openssl-dev \
    curl \
    ca-certificates \
    cmake \
    git \
    make \
    tar \
    libssl-dev \
    libsasl2-dev \
    pkg-config \
    libsystemd-dev \
    zlib1g-dev \
    libpq-dev \
    postgresql-server-dev-all \
    flex \
    bison \
    libyaml-dev \
    llvm \
    # Extra dependencies for flb-wamrc
    libmlir-14-dev libclang-common-14-dev libedit-dev libpfm4-dev llvm-14-dev libpolly-14-dev && \
    apt-get clean && rm -rf /var/lib/apt/lists/* 

RUN mkdir -p /fluent-bit/bin /fluent-bit/etc /fluent-bit/log
COPY fluent-bit/ /src/fluent-bit
WORKDIR /src/fluent-bit/build/
RUN cmake -DFLB_WAMRC=On .. && \
    make && \
    install bin/fluent-bit /fluent-bit/bin/ && \
    install bin/flb-wamrc /fluent-bit/bin/
COPY fluent-bit/conf/*.conf /fluent-bit/etc/

FROM fluent-builder as fluent-runner
COPY --from=builder /filter_dp/target/wasm32-wasi/release/filter_dp.wasm .
RUN bin/flb-wamrc -o filter_dp.aot filter_dp.wasm
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


