//! Discord message builders for queue UI components.

use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateMessage, EditMessage,
};

const QUEUE_COLOR_ACTIVE: u32 = 0x0099_FF;
const QUEUE_COLOR_CLOSED: u32 = 0x8080_80;
const QUEUE_THUMBNAIL: &str = "https://static.wikia.nocookie.net/valorant/images/c/c4/We_Did_It_Team_Spray.png/revision/latest?cb=20240627151137";

const BUTTON_JOIN: &str = "join_queue";
const BUTTON_LEAVE: &str = "leave_queue";
const BUTTON_CLOSE: &str = "close_queue";
const BUTTON_OPEN: &str = "open_queue";

/// Creates action row with Join, Leave, and Close buttons.
fn create_queue_buttons(disabled: bool) -> CreateActionRow {
    let (join_id, leave_id, close_id) = if disabled {
        (
            "join_queue_disabled",
            "leave_queue_disabled",
            "close_queue_disabled",
        )
    } else {
        (BUTTON_JOIN, BUTTON_LEAVE, BUTTON_CLOSE)
    };

    CreateActionRow::Buttons(vec![
        CreateButton::new(join_id)
            .label("Join")
            .style(ButtonStyle::Primary)
            .disabled(disabled),
        CreateButton::new(leave_id)
            .label("Leave")
            .style(ButtonStyle::Danger)
            .disabled(disabled),
        CreateButton::new(close_id)
            .label("Close")
            .style(ButtonStyle::Secondary)
            .disabled(disabled),
    ])
}

/// Formats user IDs as Discord mentions.
fn format_user_mentions(user_ids: &[String]) -> String {
    user_ids
        .iter()
        .map(|id| format!("<@{id}>"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Creates queue embed with user list and optional waitlist.
fn create_queue_embed(users: &[String], waitlist: &[String], last_action: Option<&str>) -> CreateEmbed {
    let mut description = String::new();

    // Add last action if provided
    if let Some(action_text) = last_action {
        description.push_str(action_text);
        description.push_str("\n");
    }

    if users.is_empty() {
        description.push_str("No users in queue");
    } else {
        let user_list = format_user_mentions(users);
        description.push_str(&format!("### Queued users ({})\n{user_list}", users.len()));
    }

    if !waitlist.is_empty() {
        let waitlist_list = format_user_mentions(waitlist);
        description.push_str(&format!(
            "\n\n### Waitlist ({})\n{waitlist_list}",
            waitlist.len()
        ));
    }

    CreateEmbed::new()
        .title("5-stack queue")
        .color(QUEUE_COLOR_ACTIVE)
        .description(description)
        .thumbnail(QUEUE_THUMBNAIL)
}

/// Creates initial queue message for new queues.
pub fn create_initial_queue_message(users: &[String], waitlist: &[String], last_action: Option<&str>) -> CreateMessage {
    let embed = create_queue_embed(users, waitlist, last_action);
    let buttons = create_queue_buttons(false);

    CreateMessage::new()
        .add_embed(embed)
        .components(vec![buttons])
}

/// Creates interaction response for slash command.
pub fn create_initial_interaction_response(
    users: &[String],
    waitlist: &[String],
    last_action: Option<&str>,
) -> serenity::builder::CreateInteractionResponseMessage {
    let embed = create_queue_embed(users, waitlist, last_action);
    let buttons = create_queue_buttons(false);

    serenity::builder::CreateInteractionResponseMessage::new()
        .add_embed(embed)
        .components(vec![buttons])
}

/// Creates message edit for updating active queue.
pub fn create_active_queue_message(users: &[String], waitlist: &[String], last_action: Option<&str>) -> EditMessage {
    let embed = create_queue_embed(users, waitlist, last_action);
    let buttons = create_queue_buttons(false);

    EditMessage::new().embed(embed).components(vec![buttons])
}

/// Creates message edit for closed queue with Open button.
pub fn create_closed_queue_message() -> EditMessage {
    let embed = CreateEmbed::new()
        .title("5-stack queue")
        .color(QUEUE_COLOR_CLOSED)
        .description("Queue is closed")
        .thumbnail(QUEUE_THUMBNAIL);

    let buttons = CreateActionRow::Buttons(vec![
        CreateButton::new("join_queue_disabled")
            .label("Join")
            .style(ButtonStyle::Primary)
            .disabled(true),
        CreateButton::new("leave_queue_disabled")
            .label("Leave")
            .style(ButtonStyle::Danger)
            .disabled(true),
        CreateButton::new(BUTTON_OPEN)
            .label("Open")
            .style(ButtonStyle::Success),
    ]);

    EditMessage::new().embed(embed).components(vec![buttons])
}
