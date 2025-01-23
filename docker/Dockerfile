# ------------------------------------------------------------------------------
# Stage 1: Build Solana from source for ARM64
# ------------------------------------------------------------------------------
FROM --platform=linux/arm64 rust:1.69-slim-bullseye AS builder

# Install required build dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    clang \
    libssl-dev \
    pkg-config \
    git \
    curl \
    ca-certificates \
    protobuf-compiler \
    libudev-dev && \
    rm -rf /var/lib/apt/lists/*

# Clone the Solana repository and build the binaries
WORKDIR /solana
RUN git clone --branch v1.18.26 --depth 1 https://github.com/solana-labs/solana.git .
RUN ./scripts/cargo-install-all.sh /usr/local/bin

# ------------------------------------------------------------------------------
# Stage 2: Minimal runtime image with Solana binaries for ARM64
# ------------------------------------------------------------------------------
FROM --platform=linux/arm64 ubuntu:22.04 AS solana-arm64

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    libudev1 \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /root

# Copy the built Solana binaries from the builder stage
COPY --from=builder /usr/local/bin /usr/local/bin

# Expose RPC port for solana-test-validator
EXPOSE 8899

# Default entrypoint and command
ENTRYPOINT ["solana-test-validator"]
CMD ["--reset"]
