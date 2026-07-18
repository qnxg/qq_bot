FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        grep \
        tzdata \
    && rm -rf /var/lib/apt/lists/*
# 时区设置为 Asia/Shanghai
ENV TZ=Asia/Shanghai
RUN ln -snf /usr/share/zoneinfo/Asia/Shanghai /etc/localtime \
    && echo "Asia/Shanghai" > /etc/timezone
WORKDIR /app
COPY ./target/x86_64-unknown-linux-musl/release/bot-helper .
CMD ["./bot-helper"]