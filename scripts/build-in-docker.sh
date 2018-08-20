#!/bin/bash
set -eux

DOCKER_IMAGE="getsentry/rust-musl-cross:${DOCKER_TAG}"
BUILD_DIR="/work"

DOCKER_RUN_OPTS="
  -w ${BUILD_DIR}
  -v $(pwd):${BUILD_DIR}:ro
  -v $(pwd)/target:${BUILD_DIR}/target
  -v $HOME/.cargo/registry:/root/.cargo/registry
  ${DOCKER_IMAGE}
"

docker run \
  ${DOCKER_RUN_OPTS} \
  cargo build --release --target=${TARGET} --locked

# Smoke test
env | grep SENTRY_ > .env
docker run \
  --env-file=.env \
  ${DOCKER_RUN_OPTS} \
  cargo run --release --target=${TARGET} -- releases list

# Fix permissions for shared directories
USER_ID=$(id -u)
GROUP_ID=$(id -g)
sudo chown -R ${USER_ID}:${GROUP_ID} target/ $HOME/.cargo
