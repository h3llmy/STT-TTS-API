# Builder stage
FROM --platform=linux/amd64 rust:1.93-bookworm AS builder

WORKDIR /usr/src/app

# Install build dependencies required for whisper-rs and other crates
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Fix for ARM64/aarch64 compilation errors (target specific option mismatch)
# whisper.cpp's ggml backend requires these flags to correctly compile NEON/FP16 kernels on ARM host.
# This avoids the "rosetta error" by building a native ARM binary.
ENV CFLAGS="-march=armv8.2-a+fp16+dotprod"
ENV CXXFLAGS="-march=armv8.2-a+fp16+dotprod"

# Copy manifests first for caching dependencies
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies only
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY . .

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (libssl3 is cleaner than libssl-dev for runtime)
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary and necessary UI assets
COPY --from=builder /usr/src/app/target/release/stt /app/stt
COPY ./views /app/views

# Entrypoint for the native binary
CMD ["./stt"]
