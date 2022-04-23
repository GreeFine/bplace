FROM rustlang/rust:nightly-slim

COPY . .

RUN cargo build --release

CMD cargo run --release