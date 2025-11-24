# syntax=docker/dockerfile:1.4

# Build stage using cargo-chef for layer caching
FROM --platform=$BUILDPLATFORM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
WORKDIR /app
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG TARGETPLATFORM
ARG BUILDPLATFORM

# Install cross-compilation dependencies
RUN case "$TARGETPLATFORM" in \
    "linux/arm64") \
        rustup target add aarch64-unknown-linux-musl ;; \
    "linux/amd64") \
        rustup target add x86_64-unknown-linux-musl ;; \
    esac

# Copy recipe and build dependencies
COPY --from=planner /app/recipe.json recipe.json

# Determine target architecture and build dependencies
RUN TARGET=$(case "$TARGETPLATFORM" in \
        "linux/arm64") echo "aarch64-unknown-linux-musl" ;; \
        "linux/amd64") echo "x86_64-unknown-linux-musl" ;; \
        *) echo "x86_64-unknown-linux-musl" ;; \
    esac) && \
    cargo chef cook --release --target $TARGET --recipe-path recipe.json

# Copy source code and build the binary
COPY . .
RUN TARGET=$(case "$TARGETPLATFORM" in \
        "linux/arm64") echo "aarch64-unknown-linux-musl" ;; \
        "linux/amd64") echo "x86_64-unknown-linux-musl" ;; \
        *) echo "x86_64-unknown-linux-musl" ;; \
    esac) && \
    cargo build --release --target $TARGET --bin api && \
    mv target/$TARGET/release/api /app/api

# Runtime stage
FROM alpine:latest AS runtime
RUN apk add --no-cache ca-certificates tzdata

# Create non-root user
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser

# Copy binary from builder
COPY --from=builder /app/api /usr/local/bin/api

# Set ownership
RUN chown appuser:appuser /usr/local/bin/api

USER appuser
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/api", "--health"] || exit 1

ENTRYPOINT ["/usr/local/bin/api"]