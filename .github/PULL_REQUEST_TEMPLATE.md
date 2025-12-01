## Summary
- add postgres/sqlite 0001 migrations for chats, chat_histories, recap configs/subscriptions/logs, forwarded histories
- align rust models + chat_history repo to schema (text + forwarded)
- update README with migration commands

## Test plan
- cargo fmt
- cargo check
