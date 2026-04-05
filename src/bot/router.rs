use std::sync::Arc;

use teloxide::{RequestError, dispatching::DefaultKey, dptree, prelude::*};

use crate::bot::{
    commands::Command,
    context::AppContext,
    handlers::{recap::RecapHandlers, system::SystemHandlers},
    middleware,
};

pub fn build_dispatcher(
    bot: Bot,
    ctx: Arc<AppContext>,
) -> Dispatcher<Bot, RequestError, DefaultKey> {
    let commands = dptree::entry()
        .filter_command::<Command>()
        .branch(dptree::case![Command::Start].endpoint(SystemHandlers::handle_start))
        .branch(dptree::case![Command::Help].endpoint(SystemHandlers::handle_help))
        .branch(dptree::case![Command::Cancel].endpoint(SystemHandlers::handle_cancel))
        .branch(dptree::case![Command::Recap].endpoint(RecapHandlers::handle_recap))
        .branch(
            dptree::case![Command::ConfigureRecap].endpoint(RecapHandlers::handle_configure_recap),
        );

    // Message handler: record ALL messages first, then try commands
    let message_handler = Update::filter_message()
        // Use inspect to record message as side effect (doesn't affect control flow)
        .inspect(|ctx: Arc<AppContext>, msg: Message| {
            let ctx = ctx.clone();
            let msg = msg.clone();
            tokio::spawn(async move {
                middleware::record_message(ctx, msg).await;
            });
        })
        // Then try to match commands
        .branch(commands);

    let callback_handler =
        Update::filter_callback_query().endpoint(RecapHandlers::handle_callback_query);

    let handler = dptree::entry()
        .branch(message_handler)
        .branch(callback_handler);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![ctx.clone()])
        .enable_ctrlc_handler()
        .build()
}
