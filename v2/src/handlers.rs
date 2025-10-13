//! Interaction handlers for Discord commands and button clicks.

use serenity::all::{CommandInteraction, ComponentInteraction, Context, MessageId};
use serenity::builder::CreateInteractionResponseMessage;

use crate::messages;
use crate::queue::{QueueManager, QueueOperationResult};

/// Handles the `/standby` slash command to create a new queue.
/// Creates the queue, adds the command user, and sends the initial message.
pub async fn handle_standby_command(
    command: &CommandInteraction,
    ctx: &Context,
    queue_manager: &mut QueueManager,
) {
    let guild_id = command.guild_id.expect("Expected guild_id").to_string();
    let channel_id = command.channel_id.to_string();
    let user_id = command.user.id.to_string();

    if let Ok(true) = queue_manager.queue_exists(&guild_id, &channel_id) {
        let _ = command
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Queue already exists")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    // Add user to queue first (queue/waitlist data will be created)
    let (user_ids, waitlist_ids) = match queue_manager.join_queue(&guild_id, &channel_id, &user_id)
    {
        QueueOperationResult::Success {
            users, waitlist, ..
        } => (users, waitlist),
        QueueOperationResult::Error(err) => {
            eprintln!("Failed to add creator to queue: {}", err);
            let _ = command
                .create_response(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Failed to create queue")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
        _ => {
            let _ = command
                .create_response(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Failed to create queue")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    // Get last action for display
    let last_action_text = queue_manager.get_last_action();

    let _ = command
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::Message(
                messages::create_initial_interaction_response(&user_ids, &waitlist_ids, last_action_text),
            ),
        )
        .await;

    // Now store the message ID to link it to the queue data
    if let Ok(response_msg) = command.get_response(&ctx.http).await {
        if let Err(err) =
            queue_manager.create_queue(&guild_id, &channel_id, response_msg.id.get() as i64)
        {
            eprintln!("Failed to store queue message ID: {}", err);
            // Cleanup queue/waitlist data since we couldn't store the message ID
            let _ = queue_manager.close_queue(&guild_id, &channel_id);
        }
    } else {
        eprintln!("Failed to get response message");
        // Cleanup queue/waitlist data since we couldn't get the message ID
        let _ = queue_manager.close_queue(&guild_id, &channel_id);
    }
}

/// Handles the "Join" button click.
/// Adds the user to the queue/waitlist and updates the message.
pub async fn handle_join_queue(
    component: &ComponentInteraction,
    ctx: &Context,
    queue_manager: &mut QueueManager,
) {
    let guild_id = component.guild_id.expect("Expected guild_id").to_string();
    let channel_id = component.channel_id.to_string();
    let user_id = component.user.id.to_string();

    match queue_manager.join_queue(&guild_id, &channel_id, &user_id) {
        QueueOperationResult::Success {
            users,
            waitlist,
            notification,
            ..
        } => {
            if let Err(e) = update_queue_message(
                component,
                ctx,
                queue_manager,
                &guild_id,
                &channel_id,
                &users,
                &waitlist,
            )
            .await
            {
                eprintln!("Failed to update queue message: {}", e);
                return;
            }

            if let Some(notif) = notification {
                let _ = component
                    .channel_id
                    .say(&ctx.http, notif.to_message())
                    .await;
            }
        }
        QueueOperationResult::AlreadyInQueue => {}
        QueueOperationResult::Error(err) => {
            eprintln!("Error adding user to queue: {}", err);
        }
        _ => {}
    }
}

/// Handles the "Leave" button click.
/// Removes the user from queue/waitlist, promotes from waitlist if needed.
pub async fn handle_leave_queue(
    component: &ComponentInteraction,
    ctx: &Context,
    queue_manager: &mut QueueManager,
) {
    let guild_id = component.guild_id.expect("Expected guild_id").to_string();
    let channel_id = component.channel_id.to_string();
    let user_id = component.user.id.to_string();

    match queue_manager.leave_queue(&guild_id, &channel_id, &user_id) {
        QueueOperationResult::Success {
            users,
            waitlist,
            promoted_user,
            ..
        } => {
            if let Err(e) = update_queue_message(
                component,
                ctx,
                queue_manager,
                &guild_id,
                &channel_id,
                &users,
                &waitlist,
            )
            .await
            {
                eprintln!("Failed to update queue message: {}", e);
            }

            // Send notification if someone was promoted from waitlist
            if let Some(promoted_id) = promoted_user {
                let message = format!("<@{}> you're up!", promoted_id);
                let _ = component.channel_id.say(&ctx.http, message).await;
            }
            
            // If queue is now empty, close it automatically
            if users.is_empty() && waitlist.is_empty() {
                let msg_id = match queue_manager.get_message_id(&guild_id, &channel_id) {
                    Ok(Some(id)) => id,
                    _ => {
                        eprintln!("Failed to get message ID for queue");
                        return;
                    }
                };

                if let Err(e) = queue_manager.close_queue(&guild_id, &channel_id) {
                    eprintln!("Failed to close queue after last user left: {}", e);
                } else {
                    // Update the message to show queue is closed with disabled buttons
                    let edit_message = messages::create_closed_queue_message();
                    let _ = component
                        .channel_id
                        .edit_message(&ctx.http, MessageId::new(msg_id as u64), edit_message)
                        .await;
                }
            }
        }
        QueueOperationResult::NotInQueue => {}
        QueueOperationResult::Error(err) => {
            eprintln!("Error removing user from queue: {}", err);
        }
        _ => {}
    }
}

/// Handles the "Close" button click.
/// Closes the queue and updates message to show disabled buttons with "Open" option.
pub async fn handle_close_queue(
    component: &ComponentInteraction,
    ctx: &Context,
    queue_manager: &mut QueueManager,
) {
    let guild_id = component.guild_id.expect("Expected guild_id").to_string();
    let channel_id = component.channel_id.to_string();

    let msg_id = match queue_manager.get_message_id(&guild_id, &channel_id) {
        Ok(Some(id)) => id,
        _ => {
            eprintln!("Failed to get message ID for queue");
            return;
        }
    };

    match queue_manager.close_queue(&guild_id, &channel_id) {
        Ok(()) => {
            let edit_message = messages::create_closed_queue_message();
            let _ = component
                .channel_id
                .edit_message(&ctx.http, MessageId::new(msg_id as u64), edit_message)
                .await;
        }
        Err(err) => {
            eprintln!("Error closing queue: {}", err);
        }
    }
}

/// Handles the "Open" button click on closed queues.
/// Deletes the old message, creates a new queue, and adds the opener.
pub async fn handle_open_queue(
    component: &ComponentInteraction,
    ctx: &Context,
    queue_manager: &mut QueueManager,
) {
    let guild_id = component.guild_id.expect("Expected guild_id").to_string();
    let channel_id = component.channel_id.to_string();
    let user_id = component.user.id.to_string();

    if let Err(e) = component.message.delete(&ctx.http).await {
        eprintln!("Failed to delete closed queue message: {:?}", e);
    }

    // Check if queue already exists
    match queue_manager.queue_exists(&guild_id, &channel_id) {
        Ok(true) => {
            eprintln!("Queue already exists when trying to open");
            return;
        }
        Err(e) => {
            eprintln!("Failed to check queue existence: {}", e);
            return;
        }
        _ => {}
    }

    // Clean up any orphaned queue/waitlist data before opening new queue
    // This handles edge cases where data exists but no message ID was stored
    let _ = queue_manager.close_queue(&guild_id, &channel_id);

    let Ok(msg) = component
        .channel_id
        .send_message(&ctx.http, messages::create_initial_queue_message(&[], &[], None))
        .await
    else {
        eprintln!("Error sending new queue message");
        return;
    };

    // Store the message ID first
    if let Err(err) = queue_manager.create_queue(&guild_id, &channel_id, msg.id.get() as i64) {
        eprintln!("Failed to store new queue message: {}", err);
        // Try to delete the message since we can't track it
        let _ = msg.delete(&ctx.http).await;
        return;
    }

    // Add the opener to the queue
    if let QueueOperationResult::Success {
        users, waitlist, ..
    } = queue_manager.join_queue(&guild_id, &channel_id, &user_id)
    {
        // Get last action for display
        let last_action_text = queue_manager.get_last_action();

        let edit_message = messages::create_active_queue_message(&users, &waitlist, last_action_text);
        let _ = component
            .channel_id
            .edit_message(&ctx.http, msg.id, edit_message)
            .await;
    } else {
        eprintln!("Failed to add opener to queue");
        // Cleanup on failure
        let _ = queue_manager.close_queue(&guild_id, &channel_id);
        let _ = msg.delete(&ctx.http).await;
    }
}

/// Updates the queue message with current user list.
async fn update_queue_message(
    component: &ComponentInteraction,
    ctx: &Context,
    queue_manager: &mut QueueManager,
    guild_id: &str,
    channel_id: &str,
    users: &[String],
    waitlist: &[String],
) -> Result<(), String> {
    let msg_id = queue_manager
        .get_message_id(guild_id, channel_id)
        .map_err(|e| format!("Failed to get message ID: {}", e))?
        .ok_or_else(|| "No message ID found".to_string())?;

    // Get last action for display
    let last_action_text = queue_manager.get_last_action();

    let edit_message = messages::create_active_queue_message(users, waitlist, last_action_text);

    component
        .channel_id
        .edit_message(&ctx.http, MessageId::new(msg_id as u64), edit_message)
        .await
        .map_err(|e| format!("Failed to edit message: {:?}", e))?;

    Ok(())
}
