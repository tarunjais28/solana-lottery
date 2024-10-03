FROM summerwind/actions-runner:latest

ARG RUST_TOOLCHAIN_VERSION
ARG SOLANA_VERSION

RUN sudo apt update -y && \
   sudo apt-get install -y libudev-dev libusb-1.0-0-dev \
   libsecp256k1-dev clang pkg-config libssl-dev \
   iputils-ping libpq-dev git postgresql-client \
   cargo default-libmysqlclient-dev libsqlite3-dev lld && \
   cargo install diesel_cli && \
   sudo rm -rf /var/lib/apt/lists/*

RUN sudo curl -sSL -o /usr/local/bin/argocd https://github.com/argoproj/argo-cd/releases/latest/download/argocd-linux-amd64 && \
   sudo chmod +x /usr/local/bin/argocd

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain ${RUST_TOOLCHAIN_VERSION} -y

ENV PATH ${PATH}:/home/runner/.cargo/bin

RUN sh -c "$(curl -sSfL https://release.solana.com/v${SOLANA_VERSION}/install)"

ENV PATH ${PATH}:/home/runner/.local/share/solana/install/active_release/bin

