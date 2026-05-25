# Base image for fast nbis-python Linux wheel builds (Ubuntu 24.04 + Rust + OpenCV).
# Build: scripts/build-linux-baseimage.sh

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV RUSTUP_HOME=/root/.rustup
ENV CARGO_HOME=/root/.cargo
ENV PATH=/opt/nbis-build-venv/bin:/root/.cargo/bin:${PATH}

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    cmake \
    curl \
    libopencv-dev \
    pkg-config \
    python3 \
    python3-pip \
    python3-venv \
    unzip \
    zip \
  && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal

RUN python3 -m venv /opt/nbis-build-venv \
  && /opt/nbis-build-venv/bin/pip install --upgrade pip \
  && /opt/nbis-build-venv/bin/pip install "maturin>=1.5,<2.0" twine

WORKDIR /io
