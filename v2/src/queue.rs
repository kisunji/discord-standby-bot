use crate::redis_store::RedisStore;

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
            Self::OneMore => "One more".to_string(),
            Self::Ready { users } => {
                let mentions: Vec<String> = users.iter().map(|id| format!("<@{}>", id)).collect();
                format!("There are enough users for a game!\n{}", mentions.join(", "))
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
        match self.store.contains_user(guild_id, channel_id, user_id) {
            Ok(true) => return QueueOperationResult::AlreadyInQueue,
            Err(e) => return QueueOperationResult::Error(format!("Failed to check user: {e:?}")),
            Ok(false) => {}
        }

        if let Err(e) = self.store.add_user(guild_id, channel_id, user_id) {
            return QueueOperationResult::Error(format!("Failed to add user: {e:?}"));
        }

        let all_users = match self.store.get_users(guild_id, channel_id) {
            Ok(users) => users,
            Err(e) => return QueueOperationResult::Error(format!("Failed to get users: {e:?}")),
        };

        let (users, waitlist) = Self::split_queue(all_users);

        let notification = match users.len() {
            QUEUE_ALMOST_FULL => Some(QueueNotification::OneMore),
            QUEUE_FULL => Some(QueueNotification::Ready {
                users: users.clone(),
            }),
            _ => None,
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
        match self.store.contains_user(guild_id, channel_id, user_id) {
            Ok(false) => return QueueOperationResult::NotInQueue,
            Err(e) => return QueueOperationResult::Error(format!("Failed to check user: {e:?}")),
            Ok(true) => {}
        }

        let users_before = match self.store.get_users(guild_id, channel_id) {
            Ok(users) => users,
            Err(e) => return QueueOperationResult::Error(format!("Failed to get users: {e:?}")),
        };

        let user_position = users_before.iter().position(|u| u == user_id);

        if let Err(e) = self.store.remove_user(guild_id, channel_id, user_id) {
            return QueueOperationResult::Error(format!("Failed to remove user: {e:?}"));
        }

        let all_users = match self.store.get_users(guild_id, channel_id) {
            Ok(users) => users,
            Err(e) => return QueueOperationResult::Error(format!("Failed to get users: {e:?}")),
        };

        let (users, waitlist) = Self::split_queue(all_users);

        // Detect if someone was promoted from waitlist
        let promoted_user = user_position.and_then(|pos| {
            if pos < QUEUE_FULL && users.len() == QUEUE_FULL && users_before.len() > QUEUE_FULL {
                users.get(QUEUE_FULL - 1).cloned()
            } else {
                None
            }
        });

        // Check if we should send a notification after someone leaves
        let notification = match users.len() {
            QUEUE_ALMOST_FULL => Some(QueueNotification::OneMore),
            _ => None,
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

    /// Deletes all queue data for the given guild and channel.
    pub fn close_queue(&mut self, guild_id: &str, channel_id: &str) -> Result<(), String> {
        self.store
            .delete_queue(guild_id, channel_id)
            .map_err(|e| format!("Failed to delete queue: {e:?}"))
    }
}
