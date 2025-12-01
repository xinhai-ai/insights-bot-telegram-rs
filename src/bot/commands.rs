use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "snake_case",
    description = "Available commands",
    separator = " "
)]
pub enum Command {
    #[command(description = "Show welcome message")]
    Start,
    #[command(description = "Show help")]
    Help,
    #[command(description = "Cancel current operation")]
    Cancel,
    #[command(description = "Generate chat recap")]
    Recap,
    #[command(description = "Configure recap settings")]
    ConfigureRecap,
    #[command(description = "Subscribe to group recaps")]
    SubscribeRecap,
    #[command(description = "Unsubscribe from group recaps")]
    UnsubscribeRecap,
    #[command(description = "Start forwarded message collection")]
    RecapForwardedStart,
    #[command(description = "End collection and recap forwarded")]
    RecapForwarded,
}
