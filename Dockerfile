ARG BUILD_ARCH=x86_64
ARG BUILD_LIBC=musl
FROM getsentry/rust-musl-cross:$BUILD_ARCH-$BUILD_LIBC AS sentry-build

ARG BUILD_ARCH
ARG BUILD_LIBC
ENV BUILD_TARGET=$BUILD_ARCH-unknown-linux-$BUILD_LIBC
WORKDIR /work

# Build only dependencies to speed up subsequent builds
COPY Cargo.toml Cargo.lock build.rs ./
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release --target=$BUILD_TARGET --locked

# Add all sources and rebuild the actual sentry-cli
COPY src src/

RUN touch src/main.rs && cargo build --target=$BUILD_TARGET --release --features managed

# Copy the compiled binary to a target-independent location so it can be picked up later
RUN cp target/$BUILD_TARGET/release/sentry-cli /usr/local/bin/sentry-cli

FROM alpine:3.14
WORKDIR /work
RUN apk add --no-cache ca-certificates
COPY ./docker-entrypoint.sh /
COPY --from=sentry-build /usr/local/bin/sentry-cli /bin
ENTRYPOINT ["/docker-entrypoint.sh"]
