# Build the SERVER image, i.e., yagami and lawliet

FROM rust:1-trixie AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p yagami -p yagami-runtime

FROM debian:trixie-slim
COPY --from=builder \
  /app/target/release/yagami \
  /app/target/release/yagami-runtime \
  /app/target/release/hash-password \
  /usr/local/bin/
USER nobody
CMD ["yagami"]
