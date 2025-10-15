FROM debian:trixie-slim
WORKDIR /app
COPY ./target/x86_64-unknown-linux-musl/release/bot-helper .
CMD ["./bot-helper"]