use crate::entities::FeedbackDetail;

pub fn format_feedback_summary(feedback: &FeedbackDetail) -> String {
    let mut s = format!(
        "#{}({})\n学号：{}\n时间：{}\n描述：{}",
        feedback.id,
        if feedback.msgs.is_empty() {
            "未回复"
        } else {
            "已回复"
        },
        feedback.stu_id.clone().unwrap_or("未提供".to_string()),
        feedback.create_time.format("%Y-%m-%d %H:%M:%S"),
        feedback.desc
    );
    if feedback.img_url.is_some() {
        s.push_str("\n（含有图片）");
    }
    s
}

pub fn format_feedback_detail(feedback: &FeedbackDetail) -> String {
    let mut s = format!(
        "#{} \n学号：{}\n时间：{}\n描述：{}",
        feedback.id,
        feedback.stu_id.as_ref().unwrap_or(&"未提供".to_string()),
        feedback.create_time.format("%Y-%m-%d %H:%M:%S"),
        feedback.desc
    );

    // 添加回复列表
    if !feedback.msgs.is_empty() {
        s.push_str("\n\n--- 回复列表 ---");
        for msg in &feedback.msgs {
            let typ = "回复";
            s.push_str(&format!(
                "\n[{}] {}: {}",
                typ,
                msg.created_at.format("%Y-%m-%d %H:%M"),
                msg.msg.as_deref().unwrap_or("")
            ));
        }
    } else {
        s.push_str("\n\n回复：(无)");
    }

    if feedback.img_url.is_some() {
        s.push_str("\n\n（含有图片）");
    }

    s
}

/// 将字符串压缩到指定长度，超出部分用 "..." 代替
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}
