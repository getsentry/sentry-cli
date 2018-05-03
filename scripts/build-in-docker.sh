#!/bin/bash
set -ex

BUILD_DIR="/work"

docker run \
        -w ${BUILD_DIR} \
        -v `pwd`:${BUILD_DIR}:ro \
        -v `pwd`/target:${BUILD_DIR}/target \
        -v $HOME/.cargo/registry:/root/.cargo/registry \
        -it ${DOCKER_IMAGE} \
        cargo build --release --target=${TARGET} --locked

# Fix permissions for shared directories
USER_ID=$(id -u)
GROUP_ID=$(id -g)
sudo chown -R ${USER_ID}:${GROUP_ID} target/ $HOME/.cargo
