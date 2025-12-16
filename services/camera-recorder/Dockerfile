# Use Rust nightly for edition2024 support (required by base64ct)
FROM rustlang/rust:nightly-bookworm

# Install FFmpeg
RUN apt-get update && \
    apt-get install -y ffmpeg ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy source
COPY . .

# Build
RUN cargo build --release

# Create non-root user
RUN useradd -m -u 1000 recorder && \
    mkdir -p /tmp/camera-recordings && \
    chown -R recorder:recorder /tmp/camera-recordings

USER recorder

EXPOSE 9090

ENV RUST_LOG=info
ENV TEMP_DIR=/tmp/camera-recordings

CMD ["/app/target/release/camera-recorder"]
