# Deploy 插件

通过 QQ 群指令触发 Docker 滚动发布与回滚，实现蓝绿部署与 Caddy 流量切换。

## 使用说明

- 必须 **@ 机器人**，且在配置的部署群内才会处理消息
- 仅 **管理员 QQ**（`admin_qq` 列表）可触发部署 / 回滚
- 同一时间只能执行一次部署或回滚，若已有任务进行中会提示稍后重试

## 指令列表

| 指令 | 用法 | 说明 |
| ---- | ---- | ---- |
| 部署 | `部署` | 检查镜像更新并执行滚动发布 |
| 回滚 | `回滚` | 切回上一版本（优先重启旧槽，否则用 older 镜像） |

## 部署流程

1. 比较本地与远端镜像，若无更新则直接返回
2. 拉取远端镜像（支持重试）
3. 更新本地镜像标签（`local` → `older`，`remote` → `local`）
4. 在空闲槽位（a/b）启动新容器
5. 等待容器健康检查通过
6. 更新 Caddy 配置并 reload，切换流量
7. 停止旧槽位容器（只 stop，不 rm，便于后续回滚重启）

## 回滚流程

1. 检查 `older_image` 是否存在，否则提示无可回滚版本
2. 解析当前活跃槽，对侧为目标槽
3. **优先**：若目标槽容器存在且已停止，则 `docker start` 重启
4. **兜底**：否则用 `older_image` 在目标槽重新 `docker run`
5. 等待健康检查通过（失败则停掉目标槽，不切流）
6. 更新 Caddy 配置并 reload，切换流量
7. 停止原活跃槽（只 stop）
8. 交换 `local_image` ↔ `older_image` 标签，支持连续回滚在两代间切换

## 项目结构

```
plugins/deploy/
├── Cargo.toml
└── src/
    ├── lib.rs      # 插件入口，监听 @部署 / @回滚 指令
    ├── config.rs   # 配置加载
    ├── deploy.rs   # 滚动发布与回滚主流程
    ├── docker.rs   # Docker 镜像与容器操作
    └── caddy.rs    # Caddy 配置读写与 reload
```

## 配置项

在 `config.toml` 中需要配置 `[deploy]` 节，包括：

- `group_id` / `admin_qq`：部署群与管理员
- `remote_image` / `local_image` / `older_image`：镜像名称
- `project_dir`：项目目录
- `pull_retries`：拉取镜像重试次数
- `health_timeout_secs` / `health_poll_interval_secs`：健康检查参数
- `caddy_import_file` / `caddy_template`：Caddy 配置
- `[deploy.docker_run]`：容器启动参数与 a/b 双槽位配置

可参考项目根目录的 [config.example.toml](../../config.example.toml)。
