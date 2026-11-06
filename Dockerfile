# syntax=docker/dockerfile:1
FROM docker.io/library/rust:latest

RUN apt-get update && apt-get install -y --no-install-recommends \
    mingw-w64 \
    gcc-multilib \
    libdbus-1-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    libxdo-dev \
    pkg-config \
    zip \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-pc-windows-gnu
RUN rustup target add x86_64-unknown-linux-gnu
RUN rustup component add clippy rustfmt

WORKDIR /workspace
