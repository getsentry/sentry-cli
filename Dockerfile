FROM clux/muslrust:latest AS sentry-build

RUN apt-get update && apt-get install -y cmake
WORKDIR /work

# Build only dependencies to speed up subsequent builds
ADD Cargo.toml Cargo.lock build.rs ./
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release

# Add all sources and rebuild the actual sentry-cli
ADD src src/
RUN touch src/main.rs && cargo build --release --features managed

FROM alpine:3.6
WORKDIR /work

COPY --from=sentry-build /work/target/x86_64-unknown-linux-musl/release/sentry-cli /bin
CMD ["/bin/sentry-cli"]
