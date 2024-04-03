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
RUN du -sh /src

FROM fluent as fluent-wasm
WORKDIR /fluent-bit
ARG rust_profile
COPY --from=builder /filter_dp/target/wasm32-wasi/$rust_profile/filter_dp.wasm .

FROM fluent-wasm as fluent-aot
RUN bin/flb-wamrc -o filter_dp.aot filter_dp.wasm

FROM fluent-${wasm_optimization} as fluent-runner
COPY input input
COPY filters filters
RUN mkdir output
ARG wasm_optimization
COPY fluent-bit-wrapper.sh .
RUN bash fluent-bit-wrapper.sh
# RUN bin/fluent-bit -v --dry-run \
#     -i dummy -t test1 \
#         -p "Dummy = {\"Example1\": 3, \"Example2\": 4, \"Example3\": 5}" \
#         -p "Samples = 3" \
#     -F wasm -m 'test*' \
#         -p "WASM_Path = filter_dp.$wasm_optimization" \
#         -p "Function_Name = filter_dp" \
#         -p "accessible_paths = filters" \
#     -o stdout -m '*'

# 3. Evaualtion stage
FROM docker.io/jupyter/minimal-notebook:latest as notebook
WORKDIR /notebook
COPY project.ipynb requirements.txt ./
RUN pip install -r requirements.txt
COPY input input/
COPY --from=fluent-runner /fluent-bit/output/ output/


