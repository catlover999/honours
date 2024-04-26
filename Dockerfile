# Build arguments
ARG wasm_optimization=aot
ARG rust_profile=release
ARG fluent_bit_version=v3.0.2

# 1. Build stages for filter_dp
FROM docker.io/rust:latest AS builder 
RUN rustup target add wasm32-wasi
WORKDIR /filter_dp
COPY filter_dp/Cargo.toml .
COPY filter_dp/src src
ARG rust_profile
RUN if [ "${rust_profile}" != "debug" ]; then \
      cargo build --profile ${rust_profile} --target wasm32-wasi; \
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
    flex \
    bison \
    libyaml-dev \
    llvm \
    # Extra dependencies for flb-wamrc
    libmlir-14-dev libclang-common-14-dev libedit-dev libpfm4-dev llvm-14-dev libpolly-14-dev && \
    apt-get clean && rm -rf /var/lib/apt/lists/* && \
    mkdir -p /fluent-bit/bin /fluent-bit/etc /fluent-bit/log
ARG fluent_bit_version
RUN git clone --depth 1 --branch=$fluent_bit_version https://github.com/fluent/fluent-bit.git /src/fluent-bit && \
    cp /src/fluent-bit/conf/*.conf /fluent-bit/etc/
WORKDIR /src/fluent-bit/build/
RUN cmake -DFLB_WAMRC=On .. && \
    make && \
    install bin/fluent-bit /fluent-bit/bin/ && \
    install bin/flb-wamrc /fluent-bit/bin/

FROM fluent as fluent-wasm
WORKDIR /fluent-bit
COPY fluent-bit.yaml .
ARG rust_profile
COPY --from=builder /filter_dp/target/wasm32-wasi/$rust_profile/filter_dp.wasm .

FROM fluent-wasm as fluent-aot
RUN bin/flb-wamrc -o filter_dp.aot filter_dp.wasm
RUN sed -i 's/wasm_path: filter_dp.wasm/wasm_path: filter_dp.aot/' fluent-bit.yaml

FROM fluent-${wasm_optimization} as fluent-runner
RUN mkdir output
COPY input input
RUN split -d -l 200 --additional-suffix=.csv input/EmployeeSalaries.csv input/EmployeeSalaries_ && \
    split -d -l 200 --additional-suffix=.csv input/StudentsPerformance.csv input/StudentsPerformance_
COPY filters filters
COPY parsers.conf .
ARG wasm_optimization
RUN bin/fluent-bit -c fluent-bit.yaml | { \
    grep -m 1 "Quit" && \
    sleep 5; \
    pkill -SIGTERM fluent-bit; \
    sleep 5; \
}

# 3. Evaualtion stage
FROM docker.io/jupyter/scipy-notebook:latest
COPY project.ipynb .
USER root
RUN chown jovyan:users project.ipynb
USER jovyan
COPY input input
COPY --from=fluent-runner /fluent-bit/output/ output

