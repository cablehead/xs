# Environment for compiling all executables to benchmark
FROM docker.io/chimeralinux/chimera:latest AS builder

# Update the system and install dependencies
RUN apk update --no-cache && \
    apk upgrade --no-cache && \
    apk add --no-cache bsdtar curl clang llvm gmake git zlib-ng-compat-devel openssl-devel libgit2 && \
    curl https://musl.rs/install.sh | sh

# uWebSockets for whatever reason defaults to g++ unless we set this
ENV CC clang
ENV CXX clang++

WORKDIR /build

# Compile fastwebsockets
RUN git clone https://github.com/denoland/fastwebsockets.git && \
    cd fastwebsockets && \
    cargo update && \
    cargo build --release --example echo_server --features upgrade && \
    cp target/release/examples/echo_server /usr/bin/fastwebsockets && \
    cd ..

# Compile uWebSockets
RUN git clone --recursive https://github.com/uNetworking/uWebSockets.git && \
    make -C uWebSockets && \
    cp uWebSockets/EchoServer /usr/bin/uWebSockets

# Compile tokio-tungstenite
RUN git clone https://github.com/snapview/tokio-tungstenite.git && \
    cd tokio-tungstenite && \
    cargo update && \
    sed -i.bak 's/tokio::main/tokio::main(flavor = "current_thread")/g' examples/echo-server.rs && \
    cargo build --release --example echo-server && \
    cp target/release/examples/echo-server /usr/bin/tokio-tungstenite && \
    cd ..

# Compile the benchmark tool
RUN cd /build/uWebSockets && \
    sed -i.bak "s/	g++/	clang++/g" benchmarks/Makefile && \
    sed -i.bak '5i\'$'\n'"#include <endian.h>" benchmarks/parser.cpp && \
    make -C benchmarks && \
    cp benchmarks/load_test /usr/bin/load_test

# Compile tokio-websockets
WORKDIR /build/tokio-websockets
COPY . .
RUN cargo build --release --no-default-features --features nightly,server,sha1_smol --example echo_server --target $(uname -m)-unknown-linux-musl && \
    cp target/$(uname -m)-unknown-linux-musl/release/examples/echo_server /usr/bin/tokio-websockets && \
    cd ..

# Environment for benchmarking all executables
FROM docker.io/chimeralinux/chimera:latest

RUN apk update --no-cache && \
    apk upgrade --no-cache && \
    apk add python --no-cache && \
    apk add clang python-pip python-devel --virtual .deps && \
    pip install matplotlib --break-system-packages && \
    apk del .deps

COPY --from=builder /usr/bin/fastwebsockets /usr/bin/fastwebsockets
COPY --from=builder /usr/bin/uWebSockets /usr/bin/uWebSockets
COPY --from=builder /usr/bin/tokio-tungstenite /usr/bin/tokio-tungstenite
COPY --from=builder /usr/bin/tokio-websockets /usr/bin/tokio-websockets
COPY --from=builder /usr/bin/load_test /usr/bin/load_test

WORKDIR /benches

ENV HOME /tmp

CMD ["python", "run.py"]
