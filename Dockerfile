FROM rust:1.91.1-alpine3.20@sha256:79a1bf22657dda835c5e2cf08e3aba4099ed36af3f28167103feb688a3e2604b AS sentry-build

# Install build dependencies
RUN apk add musl-dev perl openssl-dev make

WORKDIR /work

COPY apple-catalog-parsing apple-catalog-parsing/

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

FROM alpine:3.22.0@sha256:8a1f59ffb675680d47db6337b49d22281a139e9d709335b492be023728e11715
WORKDIR /work
RUN apk add --no-cache ca-certificates
COPY ./docker-entrypoint.sh /
COPY --from=sentry-build /usr/local/bin/sentry-cli /bin
ENTRYPOINT ["/docker-entrypoint.sh"]
