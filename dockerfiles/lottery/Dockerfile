FROM debian:bookworm-slim

RUN apt-get update -qq && \
   apt-get install -y libpq-dev libssl-dev ca-certificates && \
   rm -rf /var/lib/apt/lists/*

ARG binary_path
ARG binary_name

RUN mkdir /app
WORKDIR /app
COPY ${binary_path}/${binary_name} .

