# Claude Instructions for discord-standby-bot

## Deployment

When asked to deploy this project, run:
```bash
fly deploy --ha=false --depot=false
```

## Project Structure

This is a Discord bot written in Rust using the Serenity framework.

- **Main entry point**: `src/main.rs`
- **Command handlers**: `src/handlers.rs`
- **Queue logic**: `src/queue.rs`
- **Redis storage**: `src/redis_store.rs`
- **Message formatting**: `src/messages.rs`
- **Configuration**: `src/config.rs`

## Build Commands

- Build: `cargo build`
- Build release: `cargo build --release`
- Run locally: `cargo run`

## Features

- `/standby` - Create a 5-player queue
- `/bump` - Bump the queue message to bottom of chat
- `/kick <username>` - Kick a user from the queue (with autocomplete)
- Join/Leave/Close buttons on queue messages
- Automatic queue closure when empty
- Waitlist support for 6+ users
