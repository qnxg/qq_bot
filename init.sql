-- SQLite 数据库初始化脚本
-- 用于创建反馈系统所需的表结构

-- 启用 WAL 模式以提高并发性能
PRAGMA journal_mode = WAL;

-- 创建快速回复表
CREATE TABLE IF NOT EXISTS fast_reply (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 创建反馈记录表
CREATE TABLE IF NOT EXISTS feedbacks (
    feedback_id INTEGER UNSIGNED,
    qqbot_msg_id INTEGER UNSIGNED
);

-- 创建索引以提高查询性能
CREATE INDEX IF NOT EXISTS idx_feedbacks_qqbot_msg_id ON feedbacks(qqbot_msg_id);

-- 插入一些示例数据（可选）
-- INSERT OR IGNORE INTO fast_reply (id, content) VALUES 
--     ('hello', '你好！有什么可以帮助你的吗？'),
--     ('thanks', '不客气！'),
--     ('help', '可以发送 "help" 查看帮助信息');

