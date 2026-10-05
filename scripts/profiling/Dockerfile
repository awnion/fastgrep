FROM debian:12

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential ca-certificates curl git linux-perf perl python3 \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo
ENV PATH=/opt/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup.sh \
    && sh /tmp/rustup.sh -y --profile minimal --default-toolchain 1.99.0 \
    && rm /tmp/rustup.sh
ARG FLAMEGRAPH_REV=41fee1f99f9276008b7cd112fca19dc3ea84ac32
RUN git init /opt/FlameGraph \
    && git -C /opt/FlameGraph remote add origin https://github.com/brendangregg/FlameGraph.git \
    && git -C /opt/FlameGraph fetch --depth 1 origin "$FLAMEGRAPH_REV" \
    && git -C /opt/FlameGraph checkout --detach FETCH_HEAD

WORKDIR /work
ENV CARGO_TARGET_DIR=/work/target
ENV CARGO_PROFILE_RELEASE_DEBUG=1 CARGO_PROFILE_RELEASE_STRIP=false
ENV RUSTFLAGS=-Cforce-frame-pointers=yes
