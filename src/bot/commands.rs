use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "snake_case",
    description = "可用指令列表",
    separator = " "
)]
pub enum Command {
    #[command(description = "顯示啟動說明")]
    Start,
    #[command(description = "顯示幫助")]
    Help,
    #[command(description = "取消當前操作")]
    Cancel,
    #[command(description = "生成聊天回顧")]
    Recap,
    #[command(description = "配置回顧行為")]
    ConfigureRecap,
    #[command(description = "訂閱群組回顧")]
    SubscribeRecap,
    #[command(description = "取消訂閱群組回顧")]
    UnsubscribeRecap,
    #[command(description = "開始轉發收集模式")]
    RecapForwardedStart,
    #[command(description = "結束轉發收集並回顧")]
    RecapForwarded,
}
