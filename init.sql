-- SQLite 数据库初始化脚本
-- local.db 为全项目共享数据库，项目启动时会自动执行本脚本

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

-- 聊天记录收集插件 ----------------------------------

-- 被监听的群列表
CREATE TABLE IF NOT EXISTS monitored_groups (
    group_id INTEGER PRIMARY KEY,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 群聊天记录表，尽可能详尽地记录每条消息
CREATE TABLE IF NOT EXISTS chat_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id INTEGER,          -- QQ 消息 ID
    group_id INTEGER NOT NULL,   -- 来源群号
    user_id INTEGER NOT NULL,    -- 发送者 QQ 号
    nickname TEXT,               -- 发送者昵称
    card TEXT,                   -- 发送者群名片
    role TEXT,                   -- 发送者群角色（owner/admin/member）
    message_type TEXT,           -- 消息类型
    sub_type TEXT,               -- 消息子类型
    raw_message TEXT,            -- 原始消息串（含 CQ 码）
    plain_text TEXT,             -- 提取出的纯文本
    human_text TEXT,             -- 人类可读文本（含 [image] 等占位）
    images TEXT,                 -- JSON 数组，记录图片原始 URL 与下载到本地的路径
    files TEXT,                  -- JSON 数组，记录文件名/ID/大小/URL 与下载到本地的路径
    message_json TEXT,           -- 完整的 OneBot 消息段 JSON
    original_json TEXT,          -- 完整的原始事件 JSON
    self_id INTEGER,             -- 收到消息的机器人登录号
    font INTEGER,                -- 字体
    msg_time INTEGER,            -- 消息事件时间戳（秒）
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 创建索引以提高查询性能
CREATE INDEX IF NOT EXISTS idx_chat_records_group_id ON chat_records(group_id);
CREATE INDEX IF NOT EXISTS idx_chat_records_user_id ON chat_records(user_id);

-- 插入一些示例数据（可选）
-- INSERT OR IGNORE INTO fast_reply (id, content) VALUES 
--     ('hello', '你好！有什么可以帮助你的吗？'),
--     ('thanks', '不客气！'),
--     ('help', '可以发送 "help" 查看帮助信息');
