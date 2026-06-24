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

/// Slash command name for bumping the queue message.
pub const COMMAND_BUMP: &str = "bump";

/// Description shown in Discord for the bump command.
pub const COMMAND_BUMP_DESC: &str = "Bump the queue message to the bottom of chat";

/// Slash command name for kicking a user from the queue.
pub const COMMAND_KICK: &str = "kick";

/// Description shown in Discord for the kick command.
pub const COMMAND_KICK_DESC: &str = "Kick a user from the queue by username";
