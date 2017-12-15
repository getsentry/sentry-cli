FROM alpine:edge AS sentry-build

RUN apk add --no-cache \
    cargo \
    cmake \
    curl-dev \
    make \
    openssl-dev \
    rust

WORKDIR /work

ENV OPENSSL_LIB_DIR=/usr/lib/
ENV OPENSSL_INCLUDE_DIR=/usr/include
ENV OPENSSL_STATIC=1

ADD Cargo.toml Cargo.lock build.rs ./
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release

ADD src src/
RUN touch src/main.rs \
    && cargo build --release --features managed \
    && mv target/release/sentry-cli /usr/local/bin

FROM alpine:3.6
WORKDIR /work

RUN apk add --no-cache libcurl llvm-libunwind libgcc
COPY --from=sentry-build /usr/local/bin/sentry-cli /bin

CMD ["/bin/sentry-cli"]
