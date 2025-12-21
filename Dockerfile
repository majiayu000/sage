# Sage Agent Dockerfile
# Multi-stage build for minimal production image

# ============================================
# Stage 1: Build
# ============================================
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig

# Create app directory
WORKDIR /app

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./
COPY crates/sage-core/Cargo.toml crates/sage-core/
COPY crates/sage-cli/Cargo.toml crates/sage-cli/
COPY crates/sage-sdk/Cargo.toml crates/sage-sdk/
COPY crates/sage-tools/Cargo.toml crates/sage-tools/

# Create dummy source files for dependency caching
RUN mkdir -p crates/sage-core/src && echo "pub fn dummy() {}" > crates/sage-core/src/lib.rs && \
    mkdir -p crates/sage-cli/src && echo "fn main() {}" > crates/sage-cli/src/main.rs && \
    mkdir -p crates/sage-sdk/src && echo "pub fn dummy() {}" > crates/sage-sdk/src/lib.rs && \
    mkdir -p crates/sage-tools/src && echo "pub fn dummy() {}" > crates/sage-tools/src/lib.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release --bin sage 2>/dev/null || true

# Remove dummy files
RUN rm -rf crates/*/src

# Copy actual source code
COPY crates/ crates/
COPY examples/ examples/

# Build the actual application
RUN cargo build --release --bin sage

# Strip the binary to reduce size
RUN strip target/release/sage

# ============================================
# Stage 2: Runtime
# ============================================
FROM alpine:3.19 AS runtime

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    libgcc

# Create non-root user for security
RUN addgroup -g 1001 -S sage && \
    adduser -S sage -u 1001 -G sage

# Create necessary directories
RUN mkdir -p /home/sage/.config/sage && \
    chown -R sage:sage /home/sage

# Copy binary from builder
COPY --from=builder /app/target/release/sage /usr/local/bin/sage

# Set ownership
RUN chown sage:sage /usr/local/bin/sage

# Switch to non-root user
USER sage

# Set working directory
WORKDIR /workspace

# Environment variables
ENV RUST_LOG=info
ENV SAGE_CONFIG_DIR=/home/sage/.config/sage

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD sage --version || exit 1

# Default command
ENTRYPOINT ["sage"]
CMD ["--help"]
