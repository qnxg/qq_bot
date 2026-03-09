# QQBot
基于 Kovi 框架的 QQ 机器人，用于处理学生反馈问题。

## 技术栈

- **Kovi**: QQ 机器人框架
- **Rust**: 编程语言
- **SQLite**: 本地数据库存储
- **RabbitMQ**: 消息队列（接收反馈）
- **Reqwest**: HTTP 客户端（调用后端 API）

## 项目结构

```
qq_bot/
├── Cargo.toml              # 工作空间配置
├── src/main.rs             # 入口文件
├── config.toml             # 配置文件
├── plugins/                # 插件目录
│   └── feedback/           # 反馈处理插件
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs          # 插件主入口
│           ├── api.rs          # 与后端 API 交互
│           ├── config.rs       # 配置加载
│           ├── database.rs     # SQLite 数据库操作
│           ├── entities.rs    # 数据结构定义
│           ├── rabbitmq.rs     # RabbitMQ 连接管理
│           ├── utils.rs       # 工具函数
│           ├── commands/      # 指令处理
│           │   ├── mod.rs         # 指令注册
│           │   ├── framework.rs   # 指令框架定义
│           │   └── handler/       # 具体指令实现
│           │       ├── mod.rs
│           │       ├── feedback.rs    # 反馈相关指令
│           │       ├── fast_reply.rs  # 快捷回复指令
│           │       └── misc.rs        # 帮助指令
```

## 模块说明

### 核心模块

| 文件 | 说明 |
|------|------|
| [lib.rs](plugins/feedback/src/lib.rs) | 插件入口，处理消息监听和反馈消息队列 |
| [api.rs](plugins/feedback/src/api.rs) | 与后端 yqwork API 交互（获取反馈、更新状态、添加回复） |
| [database.rs](plugins/feedback/src/database.rs) | 本地 SQLite 操作，存储反馈与 QQ 消息 ID 的映射 和 fast_reply|
| [config.rs](plugins/feedback/src/config.rs) | 从 config.toml 加载配置 |
| [entities.rs](plugins/feedback/src/entities.rs) | 数据结构定义 |
| [rabbitmq.rs](plugins/feedback/src/rabbitmq.rs) | RabbitMQ 连接管理 |

### 指令系统

指令系统基于命令模式实现：

- **[framework.rs](plugins/feedback/src/commands/framework.rs)** : 定义 `CommandHandler` trait
- **[mod.rs](plugins/feedback/src/commands/mod.rs)** : 注册所有可用指令
- **[handler/](plugins/feedback/src/commands/handler/)** : 实现具体指令逻辑

#### 可用指令

| 指令 | 用法 | 说明 |
|------|------|------|
| 帮助 | `帮助` | 显示帮助信息 |
| 列表 | `列表 [状态] [页码] [每页个数]` | 查看反馈列表，默认未确认 |
| 查看 | `查看 <id>` | 查看反馈详情（包括回复列表） |
| 图片 | `图片 <id>` | 查看反馈附带图片 |
| 回复 | `回复 <id> [内容]/#[快捷回复id]` | 给反馈添加回复 |
| 确认 | `确认 <id>` | 标记为已确认 |
| 解决 | `解决 <id>` | 标记为已解决 |
| 快捷回复列表 | `快捷回复` | 查看快捷回复列表 |
| 添加快捷回复 | `快捷回复添加 <关键词> <内容>` | 添加快捷回复 |
| 删除快捷回复 | `快捷回复删除 <id>` | 删除快捷回复 |
| 快捷回复详情 | `快捷回复详情 <id>` | 查看快捷回复详情 |

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

- `feedbacks` 表: 存储反馈 ID 和对应的 QQ 消息 ID
- `fast_reply` 表: 存储快捷回复

## 配置

配置文件 `config.toml` 包含：

- `[rabbitmq]`: RabbitMQ 连接配置
- `[database]`: SQLite 数据库配置
- `[feedback]`: QQ 配置
- `[yqwork]`: 后端 API 配置

## 运行

```bash
cargo run
```

## 开发

### 检查代码

```bash
cargo check -p feedback
```
