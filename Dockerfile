# Stage 1: Build from Source
FROM rust:1.83-bullseye as builder

WORKDIR /usr/src/app
# Force Cache Invalidation (Change value to force rebuild)
ARG CACHE_BUST=2
COPY . .

# Build Release (This compiles YOUR updated node.rs)
WORKDIR /usr/src/app/volt_core
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bullseye-slim

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


# Playit Removed
# RUN wget ...

# 2. Setup User
RUN useradd -m -u 1000 appuser
WORKDIR /home/appuser/app

# 3. Copy Compile Binary from Builder Stage
COPY --from=builder /usr/src/app/volt_core/target/release/volt_core /home/appuser/app/volt_core

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
CMD ["bash", "/home/appuser/app/run.sh"]


