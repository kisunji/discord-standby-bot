//! Discord standby queue bot - manages 5-player queue with waitlist system.

mod config;
mod handlers;
mod messages;
mod queue;
mod redis_store;

use serenity::all::{
    ActivityData, Client, Context, CreateCommand, EventHandler, GatewayIntents, OnlineStatus,
};
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::model::prelude::Interaction;
use tokio::sync::Mutex;
use warp::Filter;

use crate::queue::QueueManager;

/// Main event handler with shared queue manager state.
struct Handler {
    queue_manager: Mutex<QueueManager>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        ctx.set_presence(
            Some(ActivityData::custom(config::BOT_PRESENCE)),
            OnlineStatus::Online,
        );

        let _ = serenity::all::Command::create_global_command(
            &ctx.http,
            CreateCommand::new(config::COMMAND_STANDBY)
                .description(config::COMMAND_STANDBY_DESC),
        )
        .await;
        
        println!("Global slash commands registered");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                if command.data.name == config::COMMAND_STANDBY {
                    let mut queue_manager = self.queue_manager.lock().await;
                    handlers::handle_standby_command(&command, &ctx, &mut queue_manager).await;
                } else {
                    eprintln!("Unknown slash command: {}", command.data.name);
                }
            }
            Interaction::Component(component) => {
                let custom_id = &component.data.custom_id;

                let _ = component
                    .create_response(
                        &ctx.http,
                        serenity::builder::CreateInteractionResponse::Acknowledge,
                    )
                    .await;

                // Handle open_queue separately (doesn't need active queue check)
                if custom_id == "open_queue" {
                    let mut queue_manager = self.queue_manager.lock().await;
                    handlers::handle_open_queue(&component, &ctx, &mut queue_manager).await;
                    return;
                }

                // Validate this is an active queue message
                let mut queue_manager = self.queue_manager.lock().await;
                let guild_id = component.guild_id.expect("Expected guild_id").to_string();
                let channel_id = component.channel_id.to_string();

                if !queue_manager.is_active_queue(
                    &guild_id,
                    &channel_id,
                    component.message.id.get(),
                ) {
                    eprintln!("Stale queue message clicked");
                    return;
                }

                match custom_id.as_str() {
                    "join_queue" => {
                        handlers::handle_join_queue(&component, &ctx, &mut queue_manager).await;
                    }
                    "leave_queue" => {
                        handlers::handle_leave_queue(&component, &ctx, &mut queue_manager).await;
                    }
                    "close_queue" => {
                        handlers::handle_close_queue(&component, &ctx, &mut queue_manager).await;
                    }
                    _ => eprintln!("Unknown button: {custom_id}"),
                }
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let redis_url = config::redis_url();
    let store = redis_store::RedisStore::new(&redis_url).expect("Failed to connect to Redis");

    let queue_manager = QueueManager::new(store);
    let handler = Handler {
        queue_manager: Mutex::new(queue_manager),
    };

    let token = config::bot_token();
    let mut client = Client::builder(&token, GatewayIntents::GUILD_MESSAGES)
        .event_handler(handler)
        .await
        .expect("Error creating client");

    // Start health check server on port 8000 (make fly.io happy)
    tokio::spawn(async {
        let health_check = warp::path::end()
            .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));
        
        println!("Starting health check server on port 8000...");
        warp::serve(health_check).run(([0, 0, 0, 0], 8000)).await;
    });

    println!("Starting Discord bot...");

    if let Err(e) = client.start().await {
        eprintln!("Client error: {e}");
    }
}
