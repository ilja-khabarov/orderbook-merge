FROM rust:1.65 as builder

WORKDIR /usr/src

# init project folder
RUN USER=root cargo new orderbook-merge
COPY Cargo.toml Cargo.lock /usr/src/orderbook-merge/
WORKDIR /usr/src/orderbook-merge

# add musl-compatibility
RUN apt-get update && apt-get install -y musl-tools libssl-dev protobuf-compiler libprotobuf-dev
RUN rustup target add x86_64-unknown-linux-musl

# cache layer because crates.io index update takes too long
RUN cargo build --target x86_64-unknown-linux-musl --bin orderbook-server --release

# copy meaningful sources
COPY client /usr/src/orderbook-merge/client/
COPY src /usr/src/orderbook-merge/src/
COPY proto /usr/src/orderbook-merge/proto/
COPY build.rs /usr/src/orderbook-merge/build.rs

# to prevent cached build
RUN touch /usr/src/orderbook-merge/src/main.rs

RUN cargo build --target x86_64-unknown-linux-musl --release

CMD ["./target/release/orderbook-server"]

FROM alpine:3.16.0 AS runtime

COPY --from=builder /usr/src/orderbook-merge/target/x86_64-unknown-linux-musl/release/orderbook-server /usr/local/bin

EXPOSE 8080

CMD ["/usr/local/bin/orderbook-server"]
