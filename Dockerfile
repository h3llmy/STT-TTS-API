# Builder stage
FROM rust:1.93-bookworm AS builder

WORKDIR /usr/src/app

# Install build dependencies required for whisper-rs and other crates
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    clang \
    libclang-dev \
    llvm-dev \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

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
# Touch main.rs to ensure a rebuild happens
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# Create directory for models and tts cache
RUN mkdir -p /app/models /root/.cache/k
COPY --from=builder /usr/src/app/target/release/stt /app/stt
COPY ./views /app/views

# Create directory for models
RUN mkdir -p /app/models

# Set the entrypoint
CMD ["./stt"]
