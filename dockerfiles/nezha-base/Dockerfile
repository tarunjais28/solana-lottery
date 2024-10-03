FROM rust:slim-bookworm

RUN apt-get update -qq && \
   apt-get install -y libpq-dev libsqlite3-dev default-libmysqlclient-dev && \
   rm -rf /var/lib/apt/lists/* && \
   rustup toolchain install 1.74.0-x86_64-unknown-linux-gnu && \
   cargo +1.74.0 install diesel_cli

