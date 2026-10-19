FROM ubuntu:22.04
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config curl ca-certificates git binutils python3-pil \
    libasound2-dev libxcursor-dev libwayland-dev libxkbcommon-dev xvfb xauth xdotool \
    && rm -rf /var/lib/apt/lists/*
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo
ENV PATH=/opt/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup.sh \
    && sh /tmp/rustup.sh -y --default-toolchain 1.99.1 --profile minimal --component rustfmt --component clippy \
    && rm /tmp/rustup.sh
WORKDIR /work
