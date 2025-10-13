use redis::{Client, Commands, RedisResult};

/// Redis storage layer for managing Discord queue state.
///
/// Storage schema:
/// - `{guild_id}.{channel_id}` → Discord message ID (i64)
/// - `{guild_id}.{channel_id}.queue` → Ordered list of user IDs
pub struct RedisStore {
    conn: redis::Connection,
}

impl RedisStore {
    /// Creates a new Redis store connection.
    pub fn new(redis_url: &str) -> RedisResult<Self> {
        let client = Client::open(redis_url)?;
        let conn = client.get_connection().map_err(|e| {
            eprintln!("Redis connection error: {e:?}");
            e
        })?;
        Ok(Self { conn })
    }

    /// Constructs the Redis key for storing message ID.
    fn message_key(guild_id: &str, channel_id: &str) -> String {
        format!("{guild_id}.{channel_id}")
    }

    /// Constructs the Redis key for storing queue users.
    fn queue_key(guild_id: &str, channel_id: &str) -> String {
        format!("{guild_id}.{channel_id}.queue")
    }

    /// Checks if a queue exists for the given guild and channel.
    pub fn queue_exists(&mut self, guild_id: &str, channel_id: &str) -> RedisResult<bool> {
        self.conn.exists(Self::message_key(guild_id, channel_id))
    }

    /// Stores the Discord message ID associated with a queue.
    pub fn set_message_id(
        &mut self,
        guild_id: &str,
        channel_id: &str,
        message_id: i64,
    ) -> RedisResult<()> {
        self.conn
            .set(Self::message_key(guild_id, channel_id), message_id)
    }

    /// Retrieves the Discord message ID for a queue.
    pub fn get_message_id(&mut self, guild_id: &str, channel_id: &str) -> RedisResult<Option<i64>> {
        self.conn.get(Self::message_key(guild_id, channel_id))
    }

    /// Adds a user to the end of the queue.
    pub fn add_user(&mut self, guild_id: &str, channel_id: &str, user_id: &str) -> RedisResult<()> {
        self.conn
            .rpush(Self::queue_key(guild_id, channel_id), user_id)
    }

    /// Removes all occurrences of a user from the queue.
    pub fn remove_user(
        &mut self,
        guild_id: &str,
        channel_id: &str,
        user_id: &str,
    ) -> RedisResult<()> {
        self.conn
            .lrem(Self::queue_key(guild_id, channel_id), 0, user_id)
    }

    /// Retrieves all users in the queue, in order.
    pub fn get_users(&mut self, guild_id: &str, channel_id: &str) -> RedisResult<Vec<String>> {
        self.conn
            .lrange(Self::queue_key(guild_id, channel_id), 0, -1)
    }

    /// Checks if a user is in the queue.
    pub fn contains_user(
        &mut self,
        guild_id: &str,
        channel_id: &str,
        user_id: &str,
    ) -> RedisResult<bool> {
        let users = self.get_users(guild_id, channel_id)?;
        Ok(users.iter().any(|u| u == user_id))
    }

    /// Deletes all queue data (message ID and user list) for a guild/channel.
    pub fn delete_queue(&mut self, guild_id: &str, channel_id: &str) -> RedisResult<()> {
        redis::cmd("DEL")
            .arg(&Self::message_key(guild_id, channel_id))
            .arg(&Self::queue_key(guild_id, channel_id))
            .query(&mut self.conn)
    }
}
