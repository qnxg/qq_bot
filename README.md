# QQBot

基于 Kovi 框架的 QQ 机器人，用于易千内部。


## 开发前准备

项目结构：

```
qq_bot/
├── Cargo.toml              # 工作空间配置
├── src/main.rs             # 入口文件
├── config.toml             # 配置文件
├── config.example.toml     # 配置示例
├── init.sql                # 共享数据库 local.db 初始化脚本
├── kovi.plugin.toml        # Kovi 插件启用配置
├── plugins/                # 插件目录
│   ├── feedback/           # 反馈处理插件
│   ├── relay/              # 队列消息转发插件
│   ├── chat-logger/        # 群聊天记录收集插件
│   └── deploy/             # 滚动部署插件
└── Dockerfile
```

1. 设置好配置文件

   复制 `config.example.toml` 为 `config.toml`，按需填写各插件配置。

2. 初始化数据库（可选）

   项目启动时会自动创建 `local.db` 并执行 `init.sql` 初始化表结构，通常无需手动操作。
   如需提前初始化，可执行：

   ```bash
   sqlite3 local.db < init.sql
   ```

   或者使用 SQLite 命令行工具：

   ```bash
   sqlite3 local.db
   sqlite> .read init.sql
   ```

3. 更多请参考对应插件的文档

## 插件

| 插件 | 说明 | 文档 |
| ---- | ---- | ---- |
| feedback | 接收学生反馈推送，通过指令查询、回复和管理反馈 | [plugins/feedback/README.md](plugins/feedback/README.md) |
| deploy | 通过 QQ 指令触发 Docker 蓝绿滚动发布 | [plugins/deploy/README.md](plugins/deploy/README.md) |
| daily-greeting | 定时发送早中晚问候消息，上下线通知 | [plugins/daily-greeting/README.md](plugins/daily-greeting/README.md) |
| relay | 从 RabbitMQ 队列接收纯文本消息并原样转发到 QQ 群 | [plugins/relay/README.md](plugins/relay/README.md) |
| chat-logger | 收集指定群的聊天记录（含图片）到 local.db | [plugins/chat-logger/README.md](plugins/chat-logger/README.md) |

## 指令总览

所有指令均需在对应群内 **@ 机器人** 后发送。部分插件还要求发送者为管理员。

**feedback 插件**

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

> 问题 ID 可通过回复反馈消息获得，也可在指令中直接指定。

**deploy 插件**

| 指令 | 用法 | 说明 |
| ---- | ---- | ---- |
| 部署 | `部署` | 检查镜像更新并执行滚动发布（仅管理员） |

**chat-logger 插件**（仅管理员）

| 指令 | 用法 | 说明 |
| ---- | ---- | ---- |
| 监听 | `监听 <群号>` | 将群号加入监听列表，开始收集该群聊天记录 |
| 取消监听 | `取消监听 <群号>` | 将群号从监听列表移除 |
| 统计监听 | `统计监听` | 查看所有被监听群及其已收集的记录数量 |
