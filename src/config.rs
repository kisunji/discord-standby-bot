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
pub const COMMAND_KICK_DESC: &str = "Kick a user from the queue";

/// Slash command name for shaming a user.
pub const COMMAND_SHAME: &str = "shame";

/// Description shown in Discord for the shame command.
pub const COMMAND_SHAME_DESC: &str = "Publicly shame someone";

/// Discord user ID of noverlap, who coded this bot. Shaming them backfires.
pub const NOVERLAP_USER_ID: u64 = 88437130354774016;

/// Discord role ID barred from using the shame command.
pub const SHAME_BANNED_ROLE_ID: u64 = 1524589988636000326;

/// Slash command name for self-assigning a rank role.
pub const COMMAND_RANK: &str = "rank";

/// Description shown in Discord for the rank command.
pub const COMMAND_RANK_DESC: &str = "Assign yourself a rank role";

/// Custom ID for the rank select menu component.
pub const RANK_SELECT_ID: &str = "rank_select";

/// Role IDs for each self-assignable rank. Silver is intentionally absent
/// (easter egg: choosing silver assigns no role).
pub const RANK_CHALLENGER_ROLE_ID: u64 = 832255188932100126;
pub const RANK_DIAMOND_ROLE_ID: u64 = 212760128078741504;
pub const RANK_PLAT_ROLE_ID: u64 = 178349128873410560;
pub const RANK_GOLD_ROLE_ID: u64 = 166651711346311168;

/// All assignable rank role IDs, used to keep ranks mutually exclusive by
/// removing the others when a new rank is chosen.
pub const RANK_ROLE_IDS: [u64; 4] = [
    RANK_CHALLENGER_ROLE_ID,
    RANK_DIAMOND_ROLE_ID,
    RANK_PLAT_ROLE_ID,
    RANK_GOLD_ROLE_ID,
];
