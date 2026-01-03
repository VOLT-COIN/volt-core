# Use Ubuntu 22.04 (Known Good Environment)
FROM ubuntu:22.04

# Avoid prompts
ENV DEBIAN_FRONTEND=noninteractive

# 1. Install System Deps
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    git \
    python3 \
    screen \
    pkg-config \
    libssl-dev \
    automake \
    libcurl4-openssl-dev \
    libjansson-dev \
    libgmp-dev \
    cmake \
    clang

# 2. Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# 3. Install Playit
RUN curl -L https://github.com/playit-cloud/playit-agent/releases/download/v0.15.13/playit-linux-amd64 -o /usr/local/bin/playit \
    && chmod +x /usr/local/bin/playit

# 4. Setup Non-Root User (Critical for HF Spaces)
RUN useradd -m -u 1000 appuser
WORKDIR /home/appuser/app

# 5. Clone & Build Volt (Clone to clean subdir)
RUN git clone https://github.com/eslamsheref5000/volt-core.git src

# Build inside the subdir
WORKDIR /home/appuser/app/src
RUN cargo build --bin volt_core

# Move binary to logical place
RUN mv target/debug/volt_core /home/appuser/app/volt_core
WORKDIR /home/appuser/app

# Ownership Fix
RUN chown -R appuser:appuser /home/appuser

# Switch to User
USER appuser
ENV HOME=/home/appuser

# 6. Setup Run Script (As root before switching)
USER root
COPY run.sh /home/appuser/app/run.sh
# Fix Windows CRLF
RUN sed -i 's/\r$//' /home/appuser/app/run.sh
RUN chmod +x /home/appuser/app/run.sh
RUN chown appuser:appuser /home/appuser/app/run.sh

# Final User Switch
USER appuser

EXPOSE 7860
CMD ["./run.sh"]
