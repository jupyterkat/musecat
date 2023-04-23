use poise::serenity_prelude as serenity;
use songbird::SerenityInit;
use std::sync::Arc;

mod commands;
mod config;
mod handler;
mod utils;

pub struct Data {} // User data, which is stored and accessible in all command invocations

pub type Error = eyre::Report;
pub type Context<'a> = poise::Context<'a, Data, Error>;

/// Register/Unregister slash commands (botowner only)
#[poise::command(slash_command)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    simplelog::SimpleLogger::init(log::LevelFilter::Warn, simplelog::Config::default())?;
    let config_path = std::path::Path::new(".").join("config.toml");
    let config = config::init_config(&config_path).await?;

    let intents = serenity::GatewayIntents::GUILD_VOICE_STATES | serenity::GatewayIntents::GUILDS;

    let mut handler = handler::Handler {
        options: poise::FrameworkOptions {
            commands: vec![
                register(),
                commands::queueops::play(),
                commands::queueops::stop(),
                commands::queueops::next(),
                commands::queueops::shuffle(),
                commands::queueops::clear(),
                commands::queueops::remove(),
                commands::trackops::loop_current(),
                commands::trackops::stop_looping(),
                commands::trackops::resume(),
                commands::trackops::pause(),
                commands::queue::current(),
                commands::queue::queue(),
            ],
            owners: config
                .owners
                .iter()
                .map(|&thin| serenity::UserId(thin))
                .collect(),
            ..Default::default()
        },
        shard_manager: std::sync::Mutex::new(None),
    };
    poise::set_qualified_names(&mut handler.options.commands); // some setup

    let player = songbird::Songbird::serenity();
    let handler = Arc::new(handler);
    let mut client = serenity::Client::builder(&config.discord_token, intents)
        .event_handler_arc(handler.clone())
        .register_songbird_with(player.clone())
        .await?;
    *handler.shard_manager.lock().unwrap() = Some(client.shard_manager.clone());

    client.start().await?;

    Ok(())
}
