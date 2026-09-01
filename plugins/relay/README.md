# Relay 插件

从 RabbitMQ 队列接收纯文本消息，原样转发到指定的 QQ 群。

## 工作流程

```
RabbitMQ 队列 → listen → 原样发送到 QQ 群 → ack
```

- 消息体按 UTF-8 纯文本处理，不做任何解析或格式化
- QQ 消息发送成功后才 ack；发送失败时消息留在队列中，连接恢复后会重投（可能产生重复消息）

## 配置项

在 `config.toml` 的 `[relay]` 节配置：

```toml
[relay]
# 消息来源队列（消息体为纯文本，原样转发）
message_queue = "message.qqrobot"
# 转发到哪个群
group_id = ""
```

RabbitMQ 连接复用 `[rabbitmq]` 节的 `url` 配置（见 `libs/data-client`）。
