//! Configuration management for Discord bot and Redis connection.

use std::env;

/// Retrieves Discord bot token from environment variable.
pub fn bot_token() -> String {
    env::var("BOT_TOKEN").expect("BOT_TOKEN must be set")
}

/// Retrieves Redis connection URL from environment variable.
pub fn redis_url() -> String {
    env::var("REDIS_URL").expect("REDIS_URL must be set")
}

/// Bot presence message displayed in Discord.
pub const BOT_PRESENCE: &str = "Type /standby to join";

/// Slash command name for creating queues.
pub const COMMAND_STANDBY: &str = "standby";

/// Description shown in Discord for the standby command.
pub const COMMAND_STANDBY_DESC: &str = "Start a standby queue";
