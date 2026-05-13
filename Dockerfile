# ==============================================================================
# RepoLens Docker Image
# Multi-stage build for optimized image size
# ==============================================================================

# ------------------------------------------------------------------------------
# Stage 1: Builder
# Build the RepoLens binary using Rust Alpine image
# ------------------------------------------------------------------------------
FROM rust:alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    git

# Create app directory
WORKDIR /app

# Copy all source files
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY presets ./presets
COPY schemas ./schemas

# Create dummy benchmark files to satisfy Cargo.toml [[bench]] sections
# (benchmarks are excluded from Docker build for size optimization)
RUN mkdir -p benches && \
    echo 'fn main() {}' > benches/parse_benchmark.rs && \
    echo 'fn main() {}' > benches/scanner_benchmark.rs && \
    echo 'fn main() {}' > benches/rules_benchmark.rs && \
    echo 'fn main() {}' > benches/pdf_benchmark.rs

# Build the binary
RUN cargo build --release && \
    strip target/release/repolens

# ------------------------------------------------------------------------------
# Stage 2: Runtime
# Minimal Alpine image with only necessary runtime dependencies
# ------------------------------------------------------------------------------
FROM alpine:3.22

LABEL org.opencontainers.image.title="RepoLens"
LABEL org.opencontainers.image.description="A CLI tool to audit GitHub repositories for best practices, security, and compliance"
LABEL org.opencontainers.image.url="https://github.com/systm-d/repolens"
LABEL org.opencontainers.image.source="https://github.com/systm-d/repolens"
LABEL org.opencontainers.image.vendor="Delfour.co"
LABEL org.opencontainers.image.licenses="MIT"

# Install runtime dependencies
# - git: Required for repository operations
# - ca-certificates: Required for HTTPS
# - github-cli: Required for GitHub API operations
RUN apk add --no-cache \
    git \
    ca-certificates \
    github-cli

# Create non-root user for security
RUN addgroup -g 1000 repolens && \
    adduser -u 1000 -G repolens -h /home/repolens -D repolens

# Copy binary from builder stage
COPY --from=builder /app/target/release/repolens /usr/local/bin/repolens

# Copy preset files
COPY --from=builder /app/presets /usr/share/repolens/presets

# Set ownership
RUN chown -R repolens:repolens /usr/local/bin/repolens

# Switch to non-root user
USER repolens

# Set working directory (will be mounted with the repository to audit)
WORKDIR /repo

# Set entrypoint
ENTRYPOINT ["repolens"]

# Default command (show help)
CMD ["--help"]
