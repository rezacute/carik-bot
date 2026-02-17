FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy project files
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build release
RUN cargo build --release

# Final stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/carik-bot .

# Copy config
COPY config.docker.yaml ./config.yaml

# Create non-root user
RUN useradd -m -s /bin/bash carik && chown -R carik:carik /app
USER carik

# Run the bot
CMD ["./carik-bot"]
