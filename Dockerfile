ARG BUILD_TARGET_TAG=x86_64-musl

FROM messense/rust-musl-cross:$BUILD_TARGET_TAG AS sentry-build

ARG BUILD_TARGET=x86_64-unknown-linux-musl
ENV BUILD_TARGET=${BUILD_TARGET}

WORKDIR /work

# Build only dependencies to speed up subsequent builds
ADD Cargo.toml Cargo.lock build.rs ./
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release --target=$BUILD_TARGET --locked

# Add all sources and rebuild the actual sentry-cli
ADD src src/

RUN touch src/main.rs && cargo build --target=$BUILD_TARGET --release --features managed

# Copy the compiled binary to a target-independent location so it can be picked up later
RUN cp target/$BUILD_TARGET/release/sentry-cli /usr/local/bin/sentry-cli

FROM alpine:3.7
WORKDIR /work
RUN apk add --no-cache ca-certificates
COPY --from=sentry-build /usr/local/bin/sentry-cli /bin
CMD ["/bin/sentry-cli"]
