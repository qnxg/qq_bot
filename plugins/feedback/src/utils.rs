use crate::entities::FeedbackDetail;

pub fn format_feedback_summary(feedback: &FeedbackDetail) -> String {
    let mut s = format!(
        "#{}({})\n学号：{}\n时间：{}\n描述：{}",
        feedback.id,
        if feedback.comment.is_none() {
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
    format!(
        "#{} \n学号：{}\n时间：{}\n描述：{}\n\n回复：{}{}",
        feedback.id,
        feedback.stu_id.as_ref().unwrap_or(&"未提供".to_string()),
        feedback.create_time.format("%Y-%m-%d %H:%M:%S"),
        feedback.desc,
        feedback.comment.as_ref().unwrap_or(&"(未回复)".to_string()),
        if feedback.img_url.is_some() {
            "\n\n（含有图片）"
        } else {
            ""
        }
    )
}

/// %Y-%m-%d %H:%M:%S，时区为 UTC+8
pub fn get_now_time() -> String {
    let utc_now = chrono::Utc::now();
    let utc_plus_8 = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    let now = utc_now.with_timezone(&utc_plus_8);
    now.format("%Y-%m-%d %H:%M:%S").to_string()
}
