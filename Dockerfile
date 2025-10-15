FROM debian:trixie-slim
WORKDIR /app
COPY ./target/release/bot-helper .
CMD ["./bot-helper"]