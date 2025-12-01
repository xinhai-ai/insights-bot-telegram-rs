# insights-bot-telegram-rs

Telegram-only rewrite of the recap bot inspired by the original Go implementation [`insights-bot`](https://github.com/nekomeowww/insights-bot) (MIT). This Rust version focuses on recap features and drops Slack/Discord and `/smr` web crawling.

## Features (parity target from Go version)
- `/start`, `/help`, `/cancel`
- `/recap`: summarize recent group/private messages
- `/configure_recap`: configure recap mode, auto-recap toggle and frequency
- `/subscribe_recap` / `/unsubscribe_recap`: user subscriptions to group recap
- `/recap_forwarded_start` / `/recap_forwarded`: collect forwarded messages in DM then recap
- Auto recap worker: periodic recap per group based on config
- Callback handling for recap configuration and feedback (skeleton ready)

## Tech stack
- Runtime: `tokio`
- Telegram: `teloxide` (rustls)
- DB: `sqlx::AnyPool` (Postgres preferred, fallback SQLite)
- OpenAI client: `async-openai` (recap logic stubbed; ready for integration)
- Rate limiting: `governor`
- Logging: `tracing` / `tracing-subscriber`
- i18n: YAML bundles via `serde_yaml`

## Setup
1. Copy `.env` from project template and set:
   - `TELEGRAM_BOT_TOKEN`
   - `DATABASE_URL` (Postgres) or `SQLITE_PATH` (fallback)
   - `OPENAI_API_SECRET` (or `OPENAI_API_KEY`)
   - `INSIGHTS_LANG` (`en` / `zh-Hans` / `zh-Hant`)
   - `LOCALES_DIR` (defaults `./locales`)
2. Run:
   ```bash
   cargo fmt
   cargo check
   ```
3. (Optional) create DB schema via `sqlx` migrations (not included yet).

## Database behavior
- Attempts Postgres first; on failure logs warning and connects to SQLite (`sqlite://{SQLITE_PATH}`).
- Models prepared for chat histories, recap configs/subscriptions, recap logs.

### Running migrations
- Postgres: `DATABASE_URL=postgres://... sqlx migrate run`
- SQLite: `DATABASE_URL=sqlite://data/dev.db sqlx migrate run`

### Message recording
- Middleware records incoming text/caption messages into `chat_histories`; forwarded texts in private chats are also stored in `forwarded_histories` for `/recap_forwarded`.

### Important: Group Permissions
For the bot to receive and record all messages in a group (not just commands), you must do **ONE** of the following:
1. **Disable Privacy Mode** (recommended): Contact [@BotFather](https://t.me/BotFather), send `/setprivacy`, select your bot, then choose `Disable`.
2. **Make the bot a group admin**: Add the bot as an administrator in the group settings.

Without this, the bot will only receive messages that directly mention it (e.g., `/recap@your_bot`) or are replies to the bot's messages.

### Recap configuration
- `/configure_recap` shows inline buttons to toggle recap on/off, auto recap on/off, and select per-day frequency; settings are stored in `recap_configs`.

### Auto recap
- Background worker (60s tick) finds chats due for auto recap, generates recap, sends to the group and best-effort to subscribers, then updates `last_recap_at`.

### Subscriptions
- `/subscribe_recap` (in group): user subscribes to that group’s recap; `/unsubscribe_recap` cancels. Auto recap will DM subscribers the recap as well.

### Forwarded recap flow
- `/recap_forwarded_start`: in DM, tells user to forward messages; forwarded texts are stored.
- `/recap_forwarded`: summarizes stored forwarded messages and clears them. If none, prompts user to forward first.

### Known warnings
- Unused helpers/fields (webhook config, add_feedback, media/whisper placeholders) remain until later phases implement those features.

### Rate limiting
- `/recap` and `/recap_forwarded` are limited per chat+command (default 3 requests per 60s); exceeding the limit replies with a friendly notice.

## Status
- Bot/handlers/services/db scaffolding is complete.
- Recap generation fully functional with locale-aware prompts (en/zh-Hans/zh-Hant).
- Auto-migrations run on startup (no separate migration step needed).
- `/recap` now shows time selection buttons (1h, 2h, 4h, 6h, 12h, 24h) matching Go version.
- Processing indicator shown during recap generation.

## License
MIT. Based on MIT-licensed upstream `insights-bot`; this rewrite remains MIT-compatible.
