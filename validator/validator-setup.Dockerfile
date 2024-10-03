FROM node:slim
ARG SOLANA_VERSION
RUN apt-get update \
    && apt-get install -y \
    wget \
    && rm -rf /var/lib/apt/lists/*

COPY validator/setup/ /root/setup/
RUN cd /root/setup/ && npm install
COPY validator/entrypoint.sh /root/entrypoint.sh

RUN wget https://raw.githubusercontent.com/vishnubob/wait-for-it/master/wait-for-it.sh && \
    chmod +x wait-for-it.sh \
    && chmod +x /root/entrypoint.sh

ENTRYPOINT /bin/bash -c "./wait-for-it.sh validator:8899 --strict --timeout=30 -- /root/entrypoint.sh"
