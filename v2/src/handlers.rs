//! Interaction handlers for Discord commands and button clicks.

use serenity::all::{CommandInteraction, ComponentInteraction, Context, MessageId};
use serenity::builder::{
    AutocompleteChoice, CreateAutocompleteResponse, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

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

/// Handles the `/bump` slash command to bump the queue message.
/// Deletes the old message and creates a new one at the bottom.
pub async fn handle_bump_command(
    command: &CommandInteraction,
    ctx: &Context,
    queue_manager: &mut QueueManager,
) {
    let guild_id = command.guild_id.expect("Expected guild_id").to_string();
    let channel_id = command.channel_id.to_string();

    // Check if a queue exists
    if let Ok(false) = queue_manager.queue_exists(&guild_id, &channel_id) {
        let _ = command
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("No active queue to bump")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    // Get current queue state
    let all_users = match queue_manager.get_users(&guild_id, &channel_id) {
        Ok(users) => users,
        Err(e) => {
            eprintln!("Failed to get users: {}", e);
            let _ = command
                .create_response(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Failed to bump queue")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    let (queue_users, waitlist_users) = QueueManager::split_queue(all_users);

    // Get the old message ID
    let old_msg_id = match queue_manager.get_message_id(&guild_id, &channel_id) {
        Ok(Some(id)) => id,
        _ => {
            let _ = command
                .create_response(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Failed to find queue message")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    // Delete the old message
    let _ = command
        .channel_id
        .delete_message(&ctx.http, MessageId::new(old_msg_id as u64))
        .await;

    // Send acknowledgment
    let _ = command
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Queue bumped!")
                    .ephemeral(true),
            ),
        )
        .await;

    // Get last action for display
    let last_action_text = queue_manager.get_last_action();

    // Send a new message (this "bumps" it to the bottom)
    let new_msg = match command
        .channel_id
        .send_message(
            &ctx.http,
            messages::create_initial_queue_message(&queue_users, &waitlist_users, last_action_text),
        )
        .await
    {
        Ok(msg) => msg,
        Err(e) => {
            eprintln!("Failed to send new message: {:?}", e);
            return;
        }
    };

    // Update the stored message ID
    if let Err(e) = queue_manager.create_queue(&guild_id, &channel_id, new_msg.id.get() as i64) {
        eprintln!("Failed to update message ID: {}", e);
    }
}

/// Helper function to delete old notification message if it exists.
async fn delete_old_notification(
    ctx: &Context,
    queue_manager: &mut QueueManager,
    guild_id: &str,
    channel_id: &str,
) {
    if let Ok(Some(notif_msg_id)) = queue_manager.get_notification_message_id(guild_id, channel_id) {
        let channel_id_parsed = channel_id.parse::<u64>().ok();
        if let Some(channel_id_u64) = channel_id_parsed {
            let _ = ctx
                .http
                .delete_message(
                    serenity::all::ChannelId::new(channel_id_u64),
                    MessageId::new(notif_msg_id as u64),
                    None,
                )
                .await;
        }
        let _ = queue_manager.delete_notification_message_id(guild_id, channel_id);
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
            // Delete old notification message before sending new one or updating queue
            delete_old_notification(ctx, queue_manager, &guild_id, &channel_id).await;

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
                match component
                    .channel_id
                    .say(&ctx.http, notif.to_message())
                    .await
                {
                    Ok(msg) => {
                        // Store the notification message ID
                        let _ = queue_manager.set_notification_message_id(
                            &guild_id,
                            &channel_id,
                            msg.id.get() as i64,
                        );
                    }
                    Err(e) => eprintln!("Failed to send notification: {}", e),
                }
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
            notification,
        } => {
            // Delete old notification message before sending new one or updating queue
            delete_old_notification(ctx, queue_manager, &guild_id, &channel_id).await;

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

            // Send "One more" notification if queue is at 4 users
            if let Some(notif) = notification {
                match component
                    .channel_id
                    .say(&ctx.http, notif.to_message())
                    .await
                {
                    Ok(msg) => {
                        // Store the notification message ID
                        let _ = queue_manager.set_notification_message_id(
                            &guild_id,
                            &channel_id,
                            msg.id.get() as i64,
                        );
                    }
                    Err(e) => eprintln!("Failed to send notification: {}", e),
                }
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

    // Delete notification message when closing queue
    delete_old_notification(ctx, queue_manager, &guild_id, &channel_id).await;

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

/// Handles the `/kick` slash command to remove a user from the queue by username.
pub async fn handle_kick_command(
    command: &CommandInteraction,
    ctx: &Context,
    queue_manager: &mut QueueManager,
) {
    let guild_id = command.guild_id.expect("Expected guild_id").to_string();
    let channel_id = command.channel_id.to_string();

    // Check if a queue exists
    if let Ok(false) = queue_manager.queue_exists(&guild_id, &channel_id) {
        let _ = command
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("No active queue exists")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    // Get the username parameter
    let username = match command.data.options.first() {
        Some(option) => option.value.as_str().unwrap_or(""),
        None => {
            let _ = command
                .create_response(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Please provide a username")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    // Get all users in the queue
    let all_users = match queue_manager.get_users(&guild_id, &channel_id) {
        Ok(users) => users,
        Err(e) => {
            eprintln!("Failed to get users: {}", e);
            let _ = command
                .create_response(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Failed to get queue users")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    // Build a map of user IDs to usernames
    let guild_id_parsed = match guild_id.parse::<u64>() {
        Ok(id) => serenity::all::GuildId::new(id),
        Err(_) => {
            let _ = command
                .create_response(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Invalid guild ID")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    let mut user_id_map = std::collections::HashMap::new();
    for user_id_str in &all_users {
        if let Ok(uid) = user_id_str.parse::<u64>() {
            let user_id = serenity::all::UserId::new(uid);
            if let Ok(member) = guild_id_parsed.member(&ctx.http, user_id).await {
                // Try display name first, then username
                let name = if let Some(nick) = member.nick {
                    nick
                } else {
                    member.user.name.clone()
                };
                user_id_map.insert(user_id_str.clone(), name);
            }
        }
    }

    // Kick the user
    match queue_manager.kick_user(&guild_id, &channel_id, username, &user_id_map) {
        QueueOperationResult::Success {
            users,
            waitlist,
            notification,
            promoted_user,
        } => {
            // Delete old notification message before sending new one
            delete_old_notification(ctx, queue_manager, &guild_id, &channel_id).await;

            // Get the message ID to update
            let msg_id = match queue_manager.get_message_id(&guild_id, &channel_id) {
                Ok(Some(id)) => id,
                _ => {
                    let _ = command
                        .create_response(
                            &ctx.http,
                            serenity::builder::CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content("Failed to find queue message")
                                    .ephemeral(true),
                            ),
                        )
                        .await;
                    return;
                }
            };

            // Get last action for display
            let last_action_text = queue_manager.get_last_action();

            // Update the queue message
            let edit_message = messages::create_active_queue_message(&users, &waitlist, last_action_text);
            if let Err(e) = command
                .channel_id
                .edit_message(&ctx.http, MessageId::new(msg_id as u64), edit_message)
                .await
            {
                eprintln!("Failed to edit message: {:?}", e);
            }

            // Send confirmation
            let kicked_user = user_id_map
                .iter()
                .find(|(_, name)| name.to_lowercase().contains(&username.to_lowercase()))
                .map(|(_, name)| name.as_str())
                .unwrap_or(username);

            let _ = command
                .create_response(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!("Kicked {} from the queue", kicked_user))
                            .ephemeral(true),
                    ),
                )
                .await;

            // Send notification if needed
            if let Some(notif) = notification {
                match command
                    .channel_id
                    .say(&ctx.http, notif.to_message())
                    .await
                {
                    Ok(msg) => {
                        let _ = queue_manager.set_notification_message_id(
                            &guild_id,
                            &channel_id,
                            msg.id.get() as i64,
                        );
                    }
                    Err(e) => eprintln!("Failed to send notification: {}", e),
                }
            }

            // Send promoted notification if needed
            if let Some(promoted_id) = promoted_user {
                let _ = command
                    .channel_id
                    .say(&ctx.http, format!("<@{}> has been promoted from the waitlist to the queue!", promoted_id))
                    .await;
            }

            // If queue is now empty, close it automatically
            if users.is_empty() && waitlist.is_empty() {
                if let Err(e) = queue_manager.close_queue(&guild_id, &channel_id) {
                    eprintln!("Failed to close queue after last user was kicked: {}", e);
                } else {
                    // Update the message to show queue is closed with disabled buttons
                    let edit_message = messages::create_closed_queue_message();
                    let _ = command
                        .channel_id
                        .edit_message(&ctx.http, MessageId::new(msg_id as u64), edit_message)
                        .await;
                }
            }
        }
        QueueOperationResult::NotInQueue => {
            let _ = command
                .create_response(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("User is not in the queue")
                            .ephemeral(true),
                    ),
                )
                .await;
        }
        QueueOperationResult::Error(err) => {
            let _ = command
                .create_response(
                    &ctx.http,
                    serenity::builder::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content(format!("Error: {}", err))
                            .ephemeral(true),
                    ),
                )
                .await;
        }
        _ => {}
    }
}

/// Handles autocomplete for the `/kick` command username parameter.
/// Returns a list of users currently in the queue that match the typed input.
pub async fn handle_kick_autocomplete(
    autocomplete: &CommandInteraction,
    ctx: &Context,
    queue_manager: &mut QueueManager,
) {
    let guild_id = match autocomplete.guild_id {
        Some(id) => id.to_string(),
        None => {
            // Not in a guild, can't provide autocomplete
            let _ = autocomplete
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Autocomplete(CreateAutocompleteResponse::new()),
                )
                .await;
            return;
        }
    };
    let channel_id = autocomplete.channel_id.to_string();

    // Get the current input value from autocomplete
    let focused_value = autocomplete
        .data
        .autocomplete()
        .map(|opt| opt.value)
        .unwrap_or("");

    // Check if a queue exists
    let queue_exists = queue_manager
        .queue_exists(&guild_id, &channel_id)
        .unwrap_or(false);

    if !queue_exists {
        // No queue, return empty choices
        let _ = autocomplete
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Autocomplete(CreateAutocompleteResponse::new()),
            )
            .await;
        return;
    }

    // Get all users in the queue
    let all_users = match queue_manager.get_users(&guild_id, &channel_id) {
        Ok(users) => users,
        Err(_) => {
            let _ = autocomplete
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Autocomplete(CreateAutocompleteResponse::new()),
                )
                .await;
            return;
        }
    };

    // Build a list of usernames for autocomplete
    let guild_id_parsed = match guild_id.parse::<u64>() {
        Ok(id) => serenity::all::GuildId::new(id),
        Err(_) => {
            let _ = autocomplete
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Autocomplete(CreateAutocompleteResponse::new()),
                )
                .await;
            return;
        }
    };

    let mut choices = Vec::new();
    let focused_lower = focused_value.to_lowercase();

    for user_id_str in &all_users {
        if let Ok(uid) = user_id_str.parse::<u64>() {
            let user_id = serenity::all::UserId::new(uid);
            if let Ok(member) = guild_id_parsed.member(&ctx.http, user_id).await {
                // Try display name first, then username
                let display_name = if let Some(nick) = &member.nick {
                    nick.clone()
                } else {
                    member.user.name.clone()
                };

                // Filter by the focused input (case-insensitive)
                if focused_value.is_empty()
                    || display_name.to_lowercase().contains(&focused_lower)
                {
                    choices.push(AutocompleteChoice::new(
                        display_name.clone(),
                        display_name,
                    ));

                    // Discord limits autocomplete to 25 choices
                    if choices.len() >= 25 {
                        break;
                    }
                }
            }
        }
    }

    let response = CreateAutocompleteResponse::new().set_choices(choices);

    let _ = autocomplete
        .create_response(&ctx.http, CreateInteractionResponse::Autocomplete(response))
        .await;
}
