# Stage 1: Build from Source
FROM rustlang/rust:nightly as builder

WORKDIR /usr/src/app
# Force Cache Invalidation (Change value to force rebuild)
# Cache Bust Removed
# ARG CACHE_BUST=3
COPY . .

# Build Release (This compiles YOUR updated node.rs)
WORKDIR /usr/src/app/volt_core
RUN cargo build --release

# Stage 2: Runtime
FROM ubuntu:22.04

# 1. Install Runtime Dependencies (Optimized)
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    gnupg \
    ca-certificates \
    libssl-dev \
    nginx \
    wget \
    unzip \
    openssh-client \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*


# Install Bore (TCP Tunnel)
RUN wget -O /tmp/bore.tar.gz https://github.com/ekzhang/bore/releases/download/v0.5.2/bore-v0.5.2-x86_64-unknown-linux-musl.tar.gz \
    && tar -xzf /tmp/bore.tar.gz -C /usr/local/bin/ \
    && rm /tmp/bore.tar.gz \
    && chmod +x /usr/local/bin/bore

# 2. Setup User
RUN useradd -m -u 1000 appuser
WORKDIR /home/appuser/app

# 3. Copy Compile Binary from Builder Stage
COPY --from=builder /usr/src/app/target/release/volt_core /home/appuser/app/volt_core

# 4. Copy Scripts & Configs
# Assuming files are at the Container Context Root (Uploaded flat)
COPY --chown=appuser:appuser run.sh /home/appuser/app/run.sh
COPY --chown=appuser:appuser nginx.conf /home/appuser/app/nginx.conf
COPY --chown=appuser:appuser miner.html /home/appuser/app/miner.html

# 5. Fix Permissions & Line Endings
# Ensure User owns the ENTIRE directory (so it can write wallet.key)
RUN chown -R appuser:appuser /home/appuser && \
    sed -i 's/\r$//' /home/appuser/app/run.sh && \
    chmod +x /home/appuser/app/run.sh /home/appuser/app/volt_core

# 6. Switch to User
USER appuser
ENV HOME=/home/appuser

# 7. Expose Port
EXPOSE 7860

# 8. Start the Application
# Run script, then sleep forever to keep container alive for logs
CMD ["bash", "-c", "/home/appuser/app/run.sh; echo '❌ Script finished/crashed. Keeping container alive...'; sleep infinity"]


