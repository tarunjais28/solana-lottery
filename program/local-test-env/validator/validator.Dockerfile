FROM rust:1.68 as builder
ARG SOLANA_VERSION
RUN apt-get update \
    && apt-get install -y \
    libc-bin \
    libssl-dev \
    ca-certificates \
    curl \
    libudev-dev \
    pkg-config \
    zlib1g-dev \
    llvm \
    clang \
    cmake \
    make \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add rustfmt

RUN curl -sSL "https://github.com/solana-labs/solana/archive/refs/tags/v${SOLANA_VERSION}.tar.gz" -o solana.tar.gz \
    && tar -xvf solana.tar.gz \
    && cd solana-${SOLANA_VERSION} \
    && mkdir -p /usr/local/solana \
    && ./scripts/cargo-install-all.sh /usr \
    && cp -f scripts/run.sh /usr/bin/solana-run.sh \
    && cp -f fetch-spl.sh /usr/bin/ \
    && cd /usr/bin \
    && ./fetch-spl.sh

FROM debian:bullseye-slim
RUN apt-get update \
    && apt-get install -y \
    libssl-dev \
    bzip2 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/bin /usr/bin

# RPC JSON
EXPOSE 8899/tcp
# RPC pubsub
EXPOSE 8900/tcp
# entrypoint
# EXPOSE 8001/tcp
# (future) bank service
# EXPOSE 8901/tcp
# bank service
# EXPOSE 8902/tcp
# faucet
# EXPOSE 9900/tcp
# tvu
# EXPOSE 8000/udp
# gossip
# EXPOSE 8001/udp
# tvu_forwards
# EXPOSE 8002/udp
# tpu
EXPOSE 8003/udp
# tpu_forwards
# EXPOSE 8004/udp
# retransmit
# EXPOSE 8005/udp
# repair
# EXPOSE 8006/udp
# serve_repair
# EXPOSE 8007/udp
# broadcast
# EXPOSE 8008/udp
# tpu_vote
# EXPOSE 8009/udp

ENTRYPOINT [ "solana-run.sh" ]
