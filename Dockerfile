# Build arguments
ARG wasm_optimization=aot
ARG rust_profile=release

# 1. Build stages for filter_dp
FROM docker.io/rust:latest AS builder 
RUN rustup target add wasm32-wasi
WORKDIR /filter_dp
COPY filter_dp/Cargo.toml .
COPY filter_dp/src src
ARG rust_profile
RUN if [ "${rust_profile}" != "debug" ]; then \
      cargo build --${rust_profile} --target wasm32-wasi; \
    else \
      cargo build --target wasm32-wasi; \
    fi

# 2. Fluent Bit Stage
FROM debian:bookworm-slim as fluent
ENV DEBIAN_FRONTEND=noninteractive
WORKDIR /src/fluent-bit/build/
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
    apt-get clean && rm -rf /var/lib/apt/lists/* && \
    mkdir -p /fluent-bit/bin /fluent-bit/etc /fluent-bit/log
COPY fluent-bit/ /src/fluent-bit
RUN cmake -DFLB_WAMRC=On .. && \
    make && \
    install bin/fluent-bit /fluent-bit/bin/ && \
    install bin/flb-wamrc /fluent-bit/bin/
COPY fluent-bit/conf/*.conf /fluent-bit/etc/

FROM fluent as fluent-wasm
WORKDIR /fluent-bit
ARG rust_profile
COPY --from=builder /filter_dp/target/wasm32-wasi/$rust_profile/filter_dp.wasm .
COPY fluent-bit.conf .

FROM fluent-wasm as fluent-aot
RUN bin/flb-wamrc -o filter_dp.aot filter_dp.wasm
RUN sed -i 's/WASM_Path filter_dp.wasm/WASM_Path filter_dp.aot/g' fluent-bit.conf

FROM fluent-${wasm_optimization} as fluent-runner
COPY input input
COPY filters filters
RUN mkdir output
ARG wasm_optimization
RUN stdbuf -oL bin/fluent-bit -c fluent-bit.conf | { \
    grep -m 1 "Quit" && \
    pkill -SIGTERM fluent-bit; \
    sleep 5; \
}

# 3. Evaualtion stage
FROM docker.io/jupyter/minimal-notebook:latest as notebook
WORKDIR /notebook
COPY project.ipynb requirements.txt ./
RUN pip install -r requirements.txt
COPY input input
COPY --from=fluent-runner /fluent-bit/output/ output
