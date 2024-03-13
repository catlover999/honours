FROM docker.io/rust:latest AS chef 
RUN cargo install cargo-chef 
WORKDIR filter-dp

FROM chef AS planner
COPY filter-dp/ .
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --target wasm32-unknown-unknown --recipe-path recipe.json
COPY filter-dp/ .
RUN cargo build --release --target wasm32-unknown-unknown

# 2. Fluent Bit Stage
FROM docker.io/fluent/fluent-bit:latest as fluent-bit

# Copy the WASM module from the Rust build stage
COPY --from=builder target/wasm32-unknown-unknown/release/filter_dp.wasm /fluent-bit/etc/
COPY ./fluent-bit.conf /fluent-bit/etc/

VOLUME ["/input-data", "/output-data"]

ENTRYPOINT ["/fluent-bit/bin/fluent-bit"]
CMD ["-c", "/fluent-bit/etc/fluent-bit.conf"]
