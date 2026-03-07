# 数据库设置指南

## SQLite 数据库配置

本项目已从 MySQL 迁移到 SQLite。SQLite 是一个嵌入式数据库，无需安装额外的数据库服务器。

## 快速开始

1. **配置文件设置**

   `config.toml` 中的数据库配置：
   ```toml
   [database]
   max_connections = 5  # SQLite 建议使用较小的连接数，通常 5-10 足够
   database_url = "sqlite:feedback.db"  # SQLite 数据库文件路径，会自动创建
   ```

2. **数据库文件位置**

   - `database_url` 中的路径是相对于程序运行目录的
   - 例如 `sqlite:feedback.db` 会在程序运行目录创建 `feedback.db` 文件
   - 也可以使用绝对路径，如 `sqlite:/var/lib/qqbot/feedback.db`

3. **初始化数据库表**

   运行以下命令创建表结构：
   ```bash
   sqlite3 feedback.db < init.sql
   ```

   或者使用 SQLite 命令行工具：
   ```bash
   sqlite3 feedback.db
   sqlite> .read init.sql
   ```


## 从 MySQL 迁移到 SQLite

如果你有现有的 MySQL 数据，可以使用以下方法迁移：

### 方法 1: 使用转换工具

推荐使用工具如 [sqlitebrowser](https://sqlitebrowser.org/) 或在线工具进行转换。

### 方法 2: 导出并手动转换

1. 从 MySQL 导出数据：
   ```bash
   mysqldump -u username -p database_name > data.sql
   ```

2. 转换 SQL 语法：
   - 将 `AUTO_INCREMENT` 改为 `AUTOINCREMENT`
   - 将 `TINYINT` 改为 `INTEGER`
   - 将 `DATETIME` 改为 `TEXT` 或保持 `DATETIME`
   - 将 `NOW()` 改为 `CURRENT_TIMESTAMP`
   - 将 `ON DUPLICATE KEY UPDATE` 改为 `ON CONFLICT(...) DO UPDATE SET ...`

3. 导入到 SQLite：
   ```bash
   sqlite3 feedback.db < data.sql
   ```

## SQLite 特性说明

### 1. 连接池设置

SQLite 是文件型数据库，不适合高并发写入。建议：
- `max_connections` 设置为 5-10
- 使用 `WAL` 模式提高并发性能：
  ```sql
  PRAGMA journal_mode=WAL;
  ```

### 2. SQL 语法差异

主要差异：

- **插入或更新**：
  - MySQL: `INSERT ... ON DUPLICATE KEY UPDATE ...`
  - SQLite: `INSERT ... ON CONFLICT(id) DO UPDATE SET ...`

- **日期时间**：
  - MySQL: `NOW()`, `CURRENT_TIMESTAMP`
  - SQLite: `CURRENT_TIMESTAMP`, `datetime('now')`

- **自增主键**：
  - MySQL: `AUTO_INCREMENT`
  - SQLite: `AUTOINCREMENT`

### 3. 数据类型

SQLite 使用动态类型系统，支持的类型：
- `INTEGER` - 整数
- `REAL` - 浮点数
- `TEXT` - 字符串
- `BLOB` - 二进制数据

## 常用 SQLite 命令

```bash
# 打开数据库
sqlite3 feedback.db

# 查看所有表
.tables

# 查看表结构
.schema table_name

# 执行查询
SELECT * FROM feedbacks LIMIT 10;

# 退出
.quit
```

## 备份与恢复

### 备份
```bash
sqlite3 feedback.db ".backup backup.db"
```

### 恢复
```bash
sqlite3 feedback.db ".restore backup.db"
```

## 故障排除

### 1. 数据库文件被锁定
- 检查是否有其他进程正在访问数据库
- 确保 `max_connections` 设置合理

### 2. 写入性能慢
- 启用 WAL 模式：`PRAGMA journal_mode=WAL;`
- 定期执行 `VACUUM` 优化数据库

### 3. 权限问题
- 确保程序对数据库文件所在目录有读写权限

## 参考资源

- [SQLite 官方文档](https://www.sqlite.org/docs.html)
- [SQLx SQLite 支持](https://docs.rs/sqlx/latest/sqlx/sqlite/)
- [SQLite 与 MySQL 语法对比](https://www.w3schools.com/sql/sql_syntax.asp)