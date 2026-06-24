# insights-bot-telegram-rs

Telegram-only rewrite of the recap bot inspired by the original Go implementation [`insights-bot`](https://github.com/nekomeowww/insights-bot) (MIT). This Rust version focuses on recap features and drops Slack/Discord and `/smr` web crawling.

## Features (parity target from Go version)
- `/start`, `/help`, `/cancel`
- `/recap`: summarize recent group messages
- `/configure_recap`: configure recap enablement and auto-recap delivery
- Auto recap worker: periodic recap per group based on config
- Callback handling for recap configuration

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
   - `OPENAI_API_KEY`
   - `INSIGHTS_LANG` (`en` / `zh-Hans` / `zh-Hant`)
   - `LOCALES_DIR` (defaults `./locales`)
2. Run:
   ```bash
   cargo fmt
   cargo check
   ```
3. (Optional) create DB schema via `sqlx` migrations (not included yet).

## Docker
This repo ships with `docker-compose.yml` for container deployment on the external `1panel-network`.

Create a `.env` file in the repo root or set the same values in 1Panel:
- `TELEGRAM_BOT_TOKEN`
- `OPENAI_API_KEY`
- `DATABASE_URL`
- `OPENAI_API_BASE_URL` or `OPENAI_BASE_URL` or `OPENAI_API_HOST`

Recommended `DATABASE_URL` format:
`postgresql://<user>:<password>@psql:5432/<database>`

Optional defaults:
- `OPENAI_API_MODEL_NAME=gpt-5`
- `INSIGHTS_LANG=zh-Hans`
- `LOG_LEVEL=info`

Start:
```bash
docker compose up -d --build
```

Health check:
```bash
curl http://127.0.0.1:3000/health
```

The container must join `1panel-network`, and the PostgreSQL service must be reachable as `psql` on that network.

## Database behavior
- Attempts Postgres first; on failure logs warning and connects to SQLite (`sqlite://{SQLITE_PATH}`).
- Models prepared for chat histories, recap configs/subscriptions, recap logs.

### Running migrations
- Postgres: `DATABASE_URL=postgres://... sqlx migrate run`
- SQLite: `DATABASE_URL=sqlite://data/dev.db sqlx migrate run`
- This service only supports the Rust schema defined in `migrations/postgres/0001_init.sql` and `migrations/sqlite/0001_init.sql`.
- Existing PostgreSQL databases created by the Go bot are rejected at startup and must be migrated into the Rust schema before use.

### Message recording
- Middleware records incoming group text/caption messages into `chat_histories` for recap generation.

### Important: Group Permissions
For the bot to receive and record all messages in a group (not just commands), you must do **ONE** of the following:
1. **Disable Privacy Mode** (recommended): Contact [@BotFather](https://t.me/BotFather), send `/setprivacy`, select your bot, then choose `Disable`.
2. **Make the bot a group admin**: Add the bot as an administrator in the group settings.

Without this, the bot will only receive messages that directly mention it (e.g., `/recap@your_bot`) or are replies to the bot's messages.

### Recap configuration
- `/configure_recap` shows inline buttons to toggle recap on/off and auto recap on/off; settings are stored in `recap_configs`.

### Auto recap
- Background worker finds chats due for auto recap on a fixed 6-hour cadence, generates recap, sends it to the originating group, then updates `last_recap_at`.

### Migration from the Go bot

The Rust service does not run directly on the legacy Go PostgreSQL schema. Migration from the Go bot is a one-time transform into the Rust-owned schema.

#### Retained domains
- `telegram_chats` can be mapped into `chats`.
- Group `chat_histories` can be transformed into the Rust `chat_histories` table.
- Recap enablement and auto recap enablement can be derived from `telegram_chat_feature_flags` and `telegram_chat_recaps_options` and stored in `recap_configs`.
- `last_recap_at` can be carried forward into `recap_configs` when the legacy value is safe to reuse.
- `log_chat_histories_recaps` can be imported into `recap_logs` on a best-effort basis by mapping supported fields only.

#### Dropped domains
- Private recap subscriptions from `telegram_chat_auto_recaps_subscribers`
- Forwarded recap state stored outside PostgreSQL in the Go bot
- Recap feedback reactions
- Sent-message pin tracking
- Legacy recap delivery mode and per-chat recap frequency selection

#### One-time migration order
1. Stop the Go bot and take a database snapshot.
2. Start with a fresh database for the Rust service and let the Rust migrations create the supported schema.
3. Export retained entities from the Go database.
4. Transform Go records into the Rust table shapes:
   - `telegram_chats` -> `chats`
   - supported group `chat_histories` -> `chat_histories`
   - feature flags and recap options -> `recap_configs`
   - optional recap logs -> `recap_logs`
5. Skip the dropped domains listed above.
6. Start the Rust service against the migrated Rust schema and verify recap generation before decommissioning the Go deployment.

### Known warnings
- Webhook configuration and media/whisper placeholders remain outside the recap core scope.

### Rate limiting
- `/recap` is limited per chat+command (default 3 requests per 60s); exceeding the limit replies with a friendly notice.

## Status
- Bot/handlers/services/db scaffolding is complete.
- Recap generation fully functional with locale-aware prompts (en/zh-Hans/zh-Hant).
- Auto-migrations run on startup (no separate migration step needed).
- `/recap` now shows time selection buttons (1h, 2h, 4h, 6h, 12h, 24h) matching Go version.
- Processing indicator shown during recap generation.

## License
MIT. Based on MIT-licensed upstream `insights-bot`; this rewrite remains MIT-compatible.
