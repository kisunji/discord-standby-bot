use crate::queue::{QueueManager, QueueNotification, QueueOperationResult};
use crate::redis_store::RedisStore;

fn create_test_manager() -> QueueManager {
    let store = RedisStore::new("redis://localhost:6379").unwrap();
    QueueManager::new(store)
}

#[test]
fn test_join_queue_first_user() {
    let mut manager = create_test_manager();
    let result = manager.join_queue("guild1", "channel1", "user1");
    
    match result {
        QueueOperationResult::Success { users, waitlist, notification, .. } => {
            assert_eq!(users.len(), 1);
            assert_eq!(users[0], "user1");
            assert!(waitlist.is_empty());
            assert!(notification.is_none());
        }
        _ => panic!("Expected success"),
    }
}

#[test]
fn test_join_queue_one_more_notification() {
    let mut manager = create_test_manager();
    
    for i in 1..=3 {
        manager.join_queue("guild1", "channel1", &format!("user{i}"));
    }
    
    let result = manager.join_queue("guild1", "channel1", "user4");
    
    match result {
        QueueOperationResult::Success { users, notification, .. } => {
            assert_eq!(users.len(), 4);
            assert!(matches!(notification, Some(QueueNotification::OneMore)));
        }
        _ => panic!("Expected success with OneMore notification"),
    }
}

#[test]
fn test_join_queue_ready_notification() {
    let mut manager = create_test_manager();
    
    for i in 1..=4 {
        manager.join_queue("guild1", "channel1", &format!("user{i}"));
    }
    
    let result = manager.join_queue("guild1", "channel1", "user5");
    
    match result {
        QueueOperationResult::Success { users, notification, .. } => {
            assert_eq!(users.len(), 5);
            match notification {
                Some(QueueNotification::Ready { users: notif_users }) => {
                    assert_eq!(notif_users.len(), 5);
                }
                _ => panic!("Expected Ready notification"),
            }
        }
        _ => panic!("Expected success"),
    }
}

#[test]
fn test_join_queue_waitlist() {
    let mut manager = create_test_manager();
    
    for i in 1..=5 {
        manager.join_queue("guild1", "channel1", &format!("user{i}"));
    }
    
    let result = manager.join_queue("guild1", "channel1", "user6");
    
    match result {
        QueueOperationResult::Success { users, waitlist, .. } => {
            assert_eq!(users.len(), 5);
            assert_eq!(waitlist.len(), 1);
            assert_eq!(waitlist[0], "user6");
        }
        _ => panic!("Expected success"),
    }
}

#[test]
fn test_join_queue_already_in() {
    let mut manager = create_test_manager();
    
    manager.join_queue("guild1", "channel1", "user1");
    let result = manager.join_queue("guild1", "channel1", "user1");
    
    assert!(matches!(result, QueueOperationResult::AlreadyInQueue));
}

#[test]
fn test_leave_queue_not_in() {
    let mut manager = create_test_manager();
    
    let result = manager.leave_queue("guild1", "channel1", "user1");
    
    assert!(matches!(result, QueueOperationResult::NotInQueue));
}

#[test]
fn test_leave_queue_promotion() {
    let mut manager = create_test_manager();
    
    for i in 1..=6 {
        manager.join_queue("guild1", "channel1", &format!("user{i}"));
    }
    
    let result = manager.leave_queue("guild1", "channel1", "user1");
    
    match result {
        QueueOperationResult::Success { users, waitlist, promoted_user, .. } => {
            assert_eq!(users.len(), 5);
            assert_eq!(waitlist.len(), 0);
            assert_eq!(promoted_user, Some("user6".to_string()));
        }
        _ => panic!("Expected success with promotion"),
    }
}

#[test]
fn test_notification_messages() {
    let one_more = QueueNotification::OneMore;
    assert_eq!(one_more.to_message(), "One more");
    
    let ready = QueueNotification::Ready {
        users: vec!["user1".to_string(), "user2".to_string()],
    };
    let message = ready.to_message();
    assert!(message.contains("enough users"));
    assert!(message.contains("user1"));
    assert!(message.contains("user2"));
}
