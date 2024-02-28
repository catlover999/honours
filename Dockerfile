# 1. Rust Build Stage
FROM rust:latest as rust-builder

# Add wasm target
RUN rustup target add wasm32-unknown-unknown

# Copy the Rust source code into the image
WORKDIR /usr/src/myapp
COPY ./Cargo.toml ./Cargo.lock ./
COPY src ./src

# Build the Rust WASM module
RUN cargo build --release --target wasm32-unknown-unknown


# 2. Fluent Bit Stage
FROM fluent/fluent-bit:latest as fluent-bit

# Copy the WASM module from the Rust build stage
COPY --from=rust-builder /usr/src/myapp/target/wasm32-unknown-unknown/release/filter_dp.wasm /fluent-bit/etc/
COPY ./fluent-bit.conf /fluent-bit/etc/

VOLUME ["/input-data", "/output-data"]

ENTRYPOINT ["/fluent-bit/bin/fluent-bit"]
CMD ["-c", "/fluent-bit/etc/fluent-bit.conf"]
