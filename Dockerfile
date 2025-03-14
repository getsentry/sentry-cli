FROM rust:1.84-alpine AS sentry-build

# Install build dependencies
RUN apk add musl-dev perl openssl-dev make

WORKDIR /work

# Build only dependencies to speed up subsequent builds
COPY Cargo.toml Cargo.lock build.rs ./
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release --locked

# Add all sources and rebuild the actual sentry-cli
COPY src src/
RUN touch src/main.rs && cargo build --release --features managed

# Copy the compiled binary to a target-independent location so it can be picked up later
RUN cp target/release/sentry-cli /usr/local/bin/sentry-cli

FROM alpine:3.14
WORKDIR /work
RUN apk add --no-cache ca-certificates
COPY ./docker-entrypoint.sh /
COPY --from=sentry-build /usr/local/bin/sentry-cli /bin
ENTRYPOINT ["/docker-entrypoint.sh"]
