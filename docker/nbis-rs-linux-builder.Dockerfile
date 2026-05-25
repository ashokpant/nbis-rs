# Base image for fast nbis-python Linux wheel builds (Ubuntu 24.04 + Rust 1.95 + OpenCV 4.13).
# Build: scripts/build-linux-baseimage.sh

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV RUSTUP_HOME=/root/.rustup
ENV CARGO_HOME=/root/.cargo
ENV PATH=/opt/nbis-build-venv/bin:/root/.cargo/bin:${PATH}
ENV OPENCV_DIR=/usr/local/lib/cmake/opencv4
ENV PKG_CONFIG_PATH=/usr/local/lib/pkgconfig
ENV LD_LIBRARY_PATH=/usr/local/lib

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    cmake \
    curl \
    git \
    libjpeg-dev \
    libpng-dev \
    libtiff-dev \
    pkg-config \
    python3 \
    python3-pip \
    python3-venv \
    unzip \
    zip \
  && rm -rf /var/lib/apt/lists/*

COPY scripts/install-opencv-4.13-linux.sh /tmp/install-opencv-4.13-linux.sh
RUN chmod +x /tmp/install-opencv-4.13-linux.sh && /tmp/install-opencv-4.13-linux.sh

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain 1.95.0 --profile minimal

RUN python3 -m venv /opt/nbis-build-venv \
  && /opt/nbis-build-venv/bin/pip install --upgrade pip \
  && /opt/nbis-build-venv/bin/pip install "maturin>=1.5,<2.0" twine

WORKDIR /io
