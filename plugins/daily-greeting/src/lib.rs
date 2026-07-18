mod config;

use kovi::PluginBuilder as plugin;

use crate::config::CFG;

#[kovi::plugin]
async fn main() {
    let bot = plugin::get_runtime_bot();

    // 每天早8点发送"早上好"
    let bot_morning = bot.clone();
    let group_id_morning = CFG.daily_greeting.group_id.clone();
    plugin::cron("0 8 * * *", move || {
        let bot = bot_morning.clone();
        let group_id = group_id_morning.clone();
        async move {
            tracing::info!("定时消息: 早上好");
            if let Err(e) = bot
                .send_group_msg_return(group_id.parse().unwrap(), "早上好！")
                .await
            {
                tracing::error!("发送定时消息失败: {:?}", e);
            }
        }
    })
    .unwrap();

    // 每天晚8点发送"下班！"
    let bot_evening = bot.clone();
    let group_id_evening = CFG.daily_greeting.group_id.clone();
    plugin::cron("0 20 * * *", move || {
        let bot = bot_evening.clone();
        let group_id = group_id_evening.clone();
        async move {
            tracing::info!("定时消息: 下班！");
            if let Err(e) = bot
                .send_group_msg_return(group_id.parse().unwrap(), "下班！")
                .await
            {
                tracing::error!("发送定时消息失败: {:?}", e);
            }
        }
    })
    .unwrap();

    // 退出时发送"下了"
    let bot_drop = bot.clone();
    let group_id_drop = CFG.daily_greeting.group_id.clone();
    plugin::drop(move || {
        let bot = bot_drop.clone();
        let group_id = group_id_drop.clone();
        async move {
            tracing::info!("机器人下线");
            let _ = bot
                .send_group_msg_return(group_id.parse().unwrap(), "下了")
                .await;
        }
    });

    // 上线时发送"我上线了"
    tracing::info!("机器人上线");
    let _ = bot
        .send_group_msg_return(CFG.daily_greeting.group_id.parse().unwrap(), "我上线了")
        .await;
}
