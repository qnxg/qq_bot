# Feedback 插件

处理学生反馈问题：从 RabbitMQ 接收新反馈推送到 QQ 群，并通过指令与易千工作台后端 API 交互。

## 使用说明

- 必须 **@ 机器人**，且在配置的反馈群内才会处理消息
- 问题 ID 可通过 **回复** 反馈消息，或在指令中 **直接指定** 获得

## 指令列表

| 指令 | 用法 | 说明 |
| ---- | ---- | ---- |
| 帮助 | `帮助` | 显示帮助信息 |
| 列表 | `列表 [未确认/已确认/已解决] [页码] [每页个数]` | 查看反馈列表，默认未确认、第 1 页、每页 5 条 |
| 查看 | `查看 <问题 id>` | 查看反馈详情（包括回复列表） |
| 图片 | `图片 <问题 id>` | 查看反馈附带图片 |
| 回复 | `回复 <问题 id> [...回复内容] / #[快捷回复id]` | 给反馈添加回复 |
| 确认 | `确认 <问题 id>` | 标记为已确认 |
| 解决 | `解决 <问题 id> [...回复内容] / #[快捷回复id]` | 标记为已解决并可选回复 |
| 回复列表 | `回复列表` | 查看快捷回复列表 |
| 回复更新 | `回复更新 #<快捷回复id> <...快捷回复内容>` | 添加或更新快捷回复 |
| 回复删除 | `回复删除 #<快捷回复id>` | 删除快捷回复 |
| 回复详情 | `回复详情 #<快捷回复id>` | 查看快捷回复详情 |

## 项目结构

```
plugins/feedback/
├── Cargo.toml
└── src/
    ├── lib.rs          # 插件主入口
    ├── api.rs          # 与后端 API 交互
    ├── config.rs       # 配置加载
    ├── database.rs     # SQLite 数据库操作
    ├── entities.rs     # 数据结构定义
    ├── rabbitmq.rs     # RabbitMQ 连接管理
    ├── utils.rs        # 工具函数
    └── commands/       # 指令处理
        ├── mod.rs          # 指令注册
        ├── framework.rs    # 指令框架定义
        └── handler/        # 具体指令实现
            ├── mod.rs
            ├── feedback.rs    # 反馈相关指令
            ├── fast_reply.rs  # 快捷回复指令
            └── misc.rs        # 帮助指令
```

## 模块说明

| 文件 | 说明 |
| ---- | ---- |
| [lib.rs](src/lib.rs) | 插件入口，处理消息监听和反馈消息队列 |
| [api.rs](src/api.rs) | 与后端 yqwork API 交互（获取反馈、更新状态、添加回复） |
| [database.rs](src/database.rs) | 本地 SQLite 操作，存储反馈与 QQ 消息 ID 的映射和快捷回复 |
| [config.rs](src/config.rs) | 从 config.toml 加载配置 |
| [entities.rs](src/entities.rs) | 数据结构定义 |
| [rabbitmq.rs](src/rabbitmq.rs) | RabbitMQ 连接管理 |

### 指令系统

指令系统基于命令模式实现：

- **[framework.rs](src/commands/framework.rs)**：定义 `CommandHandler` trait
- **[mod.rs](src/commands/mod.rs)**：注册所有可用指令
- **[handler/](src/commands/handler/)**：实现具体指令逻辑

## 工作流程

### 接收反馈

```
RabbitMQ 队列 → listen_feedback → 发送到 QQ 群 → 存储 msg_id 映射
```

### 处理指令

```
QQ 消息 (@机器人) → 解析指令 → 执行对应 Handler → 返回结果
```

### 指令解析

1. 提取消息中的 `@` 指令
2. 解析命令名称和参数
3. 从数据库查找回复对应的反馈 ID（如果是回复消息）
4. 调用对应的 CommandHandler 处理

## 数据库

使用 SQLite 存储反馈与 QQ 消息的映射关系：

- `feedbacks` 表：存储反馈 ID 和对应的 QQ 消息 ID
- `fast_reply` 表：存储快捷回复

## 配置项

在 `config.toml` 中需要配置以下节：

- `[rabbitmq]`：RabbitMQ 连接配置
- `[database]`：SQLite 数据库配置
- `[feedback]`：QQ 群与管理员配置
- `[yqwork]`：后端 API 配置

可参考项目根目录的 [config.example.toml](../../config.example.toml)。