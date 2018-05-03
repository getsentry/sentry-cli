#!/bin/bash
set -ex

BUILD_DIR="/work"

docker run \
        -w ${BUILD_DIR} \
        -v `pwd`:${BUILD_DIR}:ro \
        -v `pwd`/target:${BUILD_DIR}/target \
        -it ${DOCKER_IMAGE} \
        cargo build --release --target=${TARGET} --locked
