FROM ubuntu:22.04

# Install Runtime Dependencies
RUN apt-get update && apt-get install -y \
    curl \
    gnupg \
    ca-certificates \
    libssl-dev \
    python3 \
    && rm -rf /var/lib/apt/lists/*

# Setup User
RUN useradd -m -u 1000 appuser
WORKDIR /home/appuser/app

# Download Bore (Free TCP Tunnel) - KEEPING for P2P redundancy
RUN curl -L https://github.com/ekzhang/bore/releases/download/v0.6.0/bore-v0.6.0-x86_64-unknown-linux-musl.tar.gz -o bore.tar.gz \
    && tar -xzf bore.tar.gz \
    && mv bore /usr/local/bin/bore \
    && chmod +x /usr/local/bin/bore \
    && rm bore.tar.gz

# Download Playit.gg (Static TCP Tunnel for Mining)
RUN curl -L https://github.com/playit-cloud/playit-agent/releases/download/v0.15.13/playit-linux-amd64 -o /usr/local/bin/playit \
    && chmod +x /usr/local/bin/playit

# Download Latest Volt Core Binary (Linux)
RUN curl -L https://github.com/eslamsheref5000/volt-core/releases/download/latest/volt_core_linux -o /app/volt_core \
    && chmod +x /app/volt_core

# Copy Helper Scripts
COPY --chown=appuser:appuser run.sh /app/
COPY --chown=appuser:appuser proxy.py /app/

# Permissions
RUN chmod +x /app/volt_core /app/run.sh && \
    chown -R appuser:appuser /app /home/appuser

# Switch to User
USER appuser
ENV HOME=/home/appuser

# Run
CMD ["/app/run.sh"]
