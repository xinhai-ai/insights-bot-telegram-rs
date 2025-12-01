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

## Status
- Bot/handlers/services/db scaffolding is in place.
- Recap generation currently placeholder; wire OpenAI prompts next.
- SQLite/Postgres migrations need to be added.

## License
MIT. Based on MIT-licensed upstream `insights-bot`; this rewrite remains MIT-compatible.
