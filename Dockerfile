# 1. Build stages for filter_dp. Using cargo-chef for optimal layering
FROM docker.io/rust:latest AS chef 
RUN cargo install cargo-chef 
RUN rustup target add wasm32-wasi
WORKDIR /filter_dp

FROM chef AS planner
COPY filter_dp/Cargo.toml filter_dp/Cargo.lock ./
COPY filter_dp/src src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /filter_dp/recipe.json recipe.json
RUN cargo chef cook --release --target wasm32-wasi --recipe-path recipe.json
COPY filter_dp/Cargo.toml filter_dp/Cargo.lock ./
COPY filter_dp/src src
RUN cargo build --release --target wasm32-wasi

# 2. Fluent Bit Stage
FROM cr.fluentbit.io/fluent/fluent-bit:latest as fluent-bit
WORKDIR /fluent-bit
COPY --from=builder /filter_dp/target/wasm32-wasi/release/filter_dp.wasm .
COPY fluent-bit.conf .
COPY input-data input-data
ENTRYPOINT ["fluent-bit", "-c", "fluent-bit.conf"]

# 3. Evaualtion stage
FROM docker.io/jupyter/minimal-notebook:latest as notebook
WORKDIR /notebook
COPY project.ipynb requirements.txt /notebook/
RUN pip install -r requirements.txt
COPY --from=fluent-bit /fluent-bit/input-data/ .


