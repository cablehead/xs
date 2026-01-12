#!/usr/bin/bash
set -euo pipefail
set -x

function cleanup() {
    kill -9 ${WSSERVER_PID} || true
}

trap cleanup TERM EXIT

target/x86_64-unknown-linux-gnu/release/examples/autobahn_server & WSSERVER_PID=$!

sleep 3

podman run --rm -it \
    -v "${PWD}/autobahn/config:/config" \
    -v "${PWD}/autobahn/reports:/reports" \
    --net host \
    --security-opt label=disable \
    --name autobahn \
    crossbario/autobahn-testsuite \
    wstest -m fuzzingclient -s /config/fuzzingclient.json
