use crate::redis_store::RedisStore;
use crate::translations;

/// Queue size threshold for "one more needed" notification.
const QUEUE_ALMOST_FULL: usize = 4;
/// Maximum size of main queue before users go to waitlist.
const QUEUE_FULL: usize = 5;

/// Result of a queue operation with updated state.
#[derive(Debug)]
pub enum QueueOperationResult {
    Success {
        /// Users in the main queue (positions 1-5).
        users: Vec<String>,
        /// Users in the waitlist (positions 6+).
        waitlist: Vec<String>,
        /// Optional notification to send to the channel.
        notification: Option<QueueNotification>,
        /// User ID that was promoted from waitlist to queue, if any.
        promoted_user: Option<String>,
    },
    /// User attempted to join but is already in the queue.
    AlreadyInQueue,
    /// User attempted to leave but is not in the queue.
    NotInQueue,
    /// Operation failed with error message.
    Error(String),
}

/// Notifications sent to Discord channel on queue milestones.
#[derive(Debug, Clone)]
pub enum QueueNotification {
    /// Queue has 4 users, one more needed.
    OneMore,
    /// Queue is full with 5 users, ready to play.
    Ready { users: Vec<String> },
}

impl QueueNotification {
    /// Converts notification to Discord message string.
    pub fn to_message(&self) -> String {
        match self {
            Self::OneMore => {
                let (translation, language) = translations::get_random_one_more();
                format!("{} (||{}||)", translation, language)
            }
            Self::Ready { users } => {
                let mentions: Vec<String> = users.iter().map(|id| format!("<@{}>", id)).collect();
                format!(
                    "There are enough users for a game!\n{}",
                    mentions.join(", ")
                )
            }
        }
    }
}

/// Manages queue operations with business logic and Redis persistence.
pub struct QueueManager {
    store: RedisStore,
    /// Last action message (formatted, ready to display, not persisted to Redis).
    last_action: Option<String>,
}

impl QueueManager {
    pub fn new(store: RedisStore) -> Self {
        Self {
            store,
            last_action: None,
        }
    }

    /// Gets the last action message.
    pub fn get_last_action(&self) -> Option<&str> {
        self.last_action.as_deref()
    }

    /// Checks if a queue exists for the given guild and channel.
    pub fn queue_exists(&mut self, guild_id: &str, channel_id: &str) -> Result<bool, String> {
        self.store
            .queue_exists(guild_id, channel_id)
            .map_err(|e| format!("Failed to check queue existence: {e:?}"))
    }

    /// Stores the Discord message ID for a queue.
    pub fn create_queue(
        &mut self,
        guild_id: &str,
        channel_id: &str,
        message_id: i64,
    ) -> Result<(), String> {
        self.store
            .set_message_id(guild_id, channel_id, message_id)
            .map_err(|e| format!("Failed to store message ID: {e:?}"))
    }

    /// Retrieves the Discord message ID for a queue.
    pub fn get_message_id(
        &mut self,
        guild_id: &str,
        channel_id: &str,
    ) -> Result<Option<i64>, String> {
        self.store
            .get_message_id(guild_id, channel_id)
            .map_err(|e| format!("Failed to get message ID: {e:?}"))
    }

    /// Stores the notification message ID.
    pub fn set_notification_message_id(
        &mut self,
        guild_id: &str,
        channel_id: &str,
        message_id: i64,
    ) -> Result<(), String> {
        self.store
            .set_notification_message_id(guild_id, channel_id, message_id)
            .map_err(|e| format!("Failed to store notification message ID: {e:?}"))
    }

    /// Retrieves the notification message ID.
    pub fn get_notification_message_id(
        &mut self,
        guild_id: &str,
        channel_id: &str,
    ) -> Result<Option<i64>, String> {
        self.store
            .get_notification_message_id(guild_id, channel_id)
            .map_err(|e| format!("Failed to get notification message ID: {e:?}"))
    }

    /// Deletes the notification message ID.
    pub fn delete_notification_message_id(
        &mut self,
        guild_id: &str,
        channel_id: &str,
    ) -> Result<(), String> {
        self.store
            .delete_notification_message_id(guild_id, channel_id)
            .map_err(|e| format!("Failed to delete notification message ID: {e:?}"))
    }

    /// Stores the promotion message ID.
    pub fn set_promotion_message_id(
        &mut self,
        guild_id: &str,
        channel_id: &str,
        message_id: i64,
    ) -> Result<(), String> {
        self.store
            .set_promotion_message_id(guild_id, channel_id, message_id)
            .map_err(|e| format!("Failed to store promotion message ID: {e:?}"))
    }

    /// Retrieves the promotion message ID.
    pub fn get_promotion_message_id(
        &mut self,
        guild_id: &str,
        channel_id: &str,
    ) -> Result<Option<i64>, String> {
        self.store
            .get_promotion_message_id(guild_id, channel_id)
            .map_err(|e| format!("Failed to get promotion message ID: {e:?}"))
    }

    /// Deletes the promotion message ID.
    pub fn delete_promotion_message_id(
        &mut self,
        guild_id: &str,
        channel_id: &str,
    ) -> Result<(), String> {
        self.store
            .delete_promotion_message_id(guild_id, channel_id)
            .map_err(|e| format!("Failed to delete promotion message ID: {e:?}"))
    }

    /// Retrieves all users in the queue and waitlist.
    pub fn get_users(&mut self, guild_id: &str, channel_id: &str) -> Result<Vec<String>, String> {
        self.store
            .get_users(guild_id, channel_id)
            .map_err(|e| format!("Failed to get users: {e:?}"))
    }

    /// Verifies if a message ID matches the stored queue message.
    pub fn is_active_queue(&mut self, guild_id: &str, channel_id: &str, message_id: u64) -> bool {
        matches!(
            self.get_message_id(guild_id, channel_id),
            Ok(Some(stored_id)) if stored_id as u64 == message_id
        )
    }

    /// Splits all users into main queue and waitlist.
    pub fn split_queue(all_users: Vec<String>) -> (Vec<String>, Vec<String>) {
        if all_users.len() > QUEUE_FULL {
            let users = all_users[..QUEUE_FULL].to_vec();
            let waitlist = all_users[QUEUE_FULL..].to_vec();
            (users, waitlist)
        } else {
            (all_users, vec![])
        }
    }

    /// Adds a user to the queue. Returns updated queue state or error.
    pub fn join_queue(
        &mut self,
        guild_id: &str,
        channel_id: &str,
        user_id: &str,
    ) -> QueueOperationResult {
        let mut all_users = match self.store.get_users(guild_id, channel_id) {
            Ok(users) => users,
            Err(e) => return QueueOperationResult::Error(format!("Failed to get users: {e:?}")),
        };

        if all_users.iter().any(|u| u == user_id) {
            return QueueOperationResult::AlreadyInQueue;
        }

        if let Err(e) = self.store.add_user(guild_id, channel_id, user_id) {
            return QueueOperationResult::Error(format!("Failed to add user: {e:?}"));
        }

        // Reflect the write locally instead of re-reading from Redis. A
        // read-after-write on a fresh connection can observe stale (empty)
        // data when reads are served by a proxy/replica, which would render
        // the queue as "No users in queue" right after someone joins.
        all_users.push(user_id.to_string());

        let (users, waitlist) = Self::split_queue(all_users);

        // Only send notifications if the user joined the main queue (not waitlist)
        // This prevents duplicate Ready notifications when someone joins position 6+
        let user_in_main_queue = users.contains(&user_id.to_string());

        let notification = if user_in_main_queue {
            match users.len() {
                QUEUE_ALMOST_FULL => Some(QueueNotification::OneMore),
                QUEUE_FULL => Some(QueueNotification::Ready {
                    users: users.clone(),
                }),
                _ => None,
            }
        } else {
            None
        };

        // Track the last action
        self.last_action = Some(format!("<@{}> joined!", user_id));

        QueueOperationResult::Success {
            users,
            waitlist,
            notification,
            promoted_user: None,
        }
    }

    /// Removes a user from the queue. Returns updated queue state or error.
    /// If a user from the waitlist gets promoted, their ID is included in the result.
    pub fn leave_queue(
        &mut self,
        guild_id: &str,
        channel_id: &str,
        user_id: &str,
    ) -> QueueOperationResult {
        let users_before = match self.store.get_users(guild_id, channel_id) {
            Ok(users) => users,
            Err(e) => return QueueOperationResult::Error(format!("Failed to get users: {e:?}")),
        };

        let user_position = match users_before.iter().position(|u| u == user_id) {
            Some(pos) => pos,
            None => return QueueOperationResult::NotInQueue,
        };

        if let Err(e) = self.store.remove_user(guild_id, channel_id, user_id) {
            return QueueOperationResult::Error(format!("Failed to remove user: {e:?}"));
        }

        // Reflect the write locally instead of re-reading from Redis to avoid
        // a stale read-after-write that could misrepresent the queue state.
        let mut all_users = users_before.clone();
        all_users.retain(|u| u != user_id);

        let (users, waitlist) = Self::split_queue(all_users);

        // Detect if someone was promoted from waitlist
        let promoted_user = if user_position < QUEUE_FULL
            && users.len() == QUEUE_FULL
            && users_before.len() > QUEUE_FULL
        {
            users.get(QUEUE_FULL - 1).cloned()
        } else {
            None
        };

        // Check if we should send a notification after someone leaves
        // Don't send notification if someone was promoted (they get their own message)
        let notification = if promoted_user.is_none() && users.len() == QUEUE_ALMOST_FULL {
            Some(QueueNotification::OneMore)
        } else {
            None
        };

        // Track the last action
        self.last_action = Some(format!("<@{}> left!", user_id));

        QueueOperationResult::Success {
            users,
            waitlist,
            notification,
            promoted_user,
        }
    }

    /// Resolves a kick query to a queued user ID.
    ///
    /// Matching is tried in order of decreasing confidence:
    /// 1. A mention (`<@id>` / `<@!id>`) or raw numeric ID that is in the queue.
    /// 2. An exact (case-insensitive) match against one of a user's names.
    /// 3. A substring match against one of a user's names.
    /// Kicks a user from the queue by their Discord user ID.
    ///
    /// Returns [`QueueOperationResult::NotInQueue`] if the user is not currently
    /// in the queue or waitlist, otherwise returns the updated queue state.
    pub fn kick_user(
        &mut self,
        guild_id: &str,
        channel_id: &str,
        user_id: &str,
    ) -> QueueOperationResult {
        let user_id = user_id.to_string();

        // Use the existing leave_queue logic
        let users_before = match self.store.get_users(guild_id, channel_id) {
            Ok(users) => users,
            Err(e) => return QueueOperationResult::Error(format!("Failed to get users: {e:?}")),
        };

        let user_position = match users_before.iter().position(|u| u == &user_id) {
            Some(pos) => pos,
            None => return QueueOperationResult::NotInQueue,
        };

        if let Err(e) = self.store.remove_user(guild_id, channel_id, &user_id) {
            return QueueOperationResult::Error(format!("Failed to remove user: {e:?}"));
        }

        // Reflect the write locally instead of re-reading from Redis to avoid
        // a stale read-after-write that could misrepresent the queue state.
        let mut all_users = users_before.clone();
        all_users.retain(|u| u != &user_id);

        let (users, waitlist) = Self::split_queue(all_users);

        // Detect if someone was promoted from waitlist
        let promoted_user = if user_position < QUEUE_FULL
            && users.len() == QUEUE_FULL
            && users_before.len() > QUEUE_FULL
        {
            users.get(QUEUE_FULL - 1).cloned()
        } else {
            None
        };

        // Check if we should send a notification after someone is kicked
        // Don't send notification if someone was promoted (they get their own message)
        let notification = if promoted_user.is_none() && users.len() == QUEUE_ALMOST_FULL {
            Some(QueueNotification::OneMore)
        } else {
            None
        };

        // Track the last action
        self.last_action = Some(format!("<@{}> was kicked!", user_id));

        QueueOperationResult::Success {
            users,
            waitlist,
            notification,
            promoted_user,
        }
    }

    /// Deletes all queue data for the given guild and channel.
    pub fn close_queue(&mut self, guild_id: &str, channel_id: &str) -> Result<(), String> {
        self.store
            .delete_queue(guild_id, channel_id)
            .map_err(|e| format!("Failed to delete queue: {e:?}"))
    }
}
