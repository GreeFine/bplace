FROM rust:1.59.0-buster

COPY . .

RUN cargo build --release

CMD cargo run --release