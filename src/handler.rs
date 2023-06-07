use super::{Data, Error};
use poise::serenity_prelude::{self as serenity, Activity};

use super::config::get_config;

pub struct Handler {
    pub options: poise::FrameworkOptions<Data, Error>,
    pub shard_manager:
        std::sync::Mutex<Option<std::sync::Arc<tokio::sync::Mutex<serenity::ShardManager>>>>,
}
#[serenity::async_trait]
impl serenity::EventHandler for Handler {
    async fn ready(&self, ctx: serenity::Context, data_about_bot: serenity::Ready) {
        if let Err(e) = poise::builtins::register_globally(&ctx.http, &self.options.commands).await
        {
            log::error!("{:?}", e)
        };

        let config = get_config();

        let application_id = ctx.http.application_id().unwrap_or(0);

        println!("Ready! Invite the bot with https://discordapp.com/oauth2/authorize?client_id={application_id}&scope=bot%20applications.commands&permissions=36700160");

        let status = match config.bot_status.as_str() {
            "online" => serenity::OnlineStatus::Online,
            "idle" => serenity::OnlineStatus::Idle,
            "donotdisturb" => serenity::OnlineStatus::DoNotDisturb,
            "offline" => serenity::OnlineStatus::Offline,
            "invisible" => serenity::OnlineStatus::Invisible,
            _ => serenity::OnlineStatus::Online,
        };

        let mut activity_string = config.bot_activity.clone();
        activity_string.truncate(127);

        let activity = match config.bot_activity_type.as_str() {
            "listening" => Activity::listening(activity_string),
            "streaming" => Activity::streaming(activity_string, config.bot_activity_url.as_str()),
            "playing" => Activity::playing(activity_string),
            "watching" => Activity::watching(activity_string),
            "competing" => Activity::competing(activity_string),
            _ => Activity::listening("music"),
        };

        ctx.set_presence(Some(activity), status).await;

        let shard_manager = (*self.shard_manager.lock().unwrap()).clone().unwrap();
        let framework_data = poise::FrameworkContext {
            bot_id: Default::default(),
            options: &self.options,
            user_data: &Data {},
            shard_manager: &shard_manager,
        };

        poise::dispatch_event(
            framework_data,
            &ctx,
            &poise::Event::Ready { data_about_bot },
        )
        .await;
    }
    async fn guild_create(&self, ctx: serenity::Context, guild: serenity::Guild, is_new: bool) {
        let shard_manager = (*self.shard_manager.lock().unwrap()).clone().unwrap();
        let framework_data = poise::FrameworkContext {
            bot_id: Default::default(),
            options: &self.options,
            user_data: &Data {},
            shard_manager: &shard_manager,
        };

        poise::dispatch_event(
            framework_data,
            &ctx,
            &poise::Event::GuildCreate { guild, is_new },
        )
        .await;
    }

    async fn interaction_create(&self, ctx: serenity::Context, interaction: serenity::Interaction) {
        let shard_manager = (*self.shard_manager.lock().unwrap()).clone().unwrap();
        let framework_data = poise::FrameworkContext {
            bot_id: Default::default(),
            options: &self.options,
            user_data: &Data {},
            shard_manager: &shard_manager,
        };

        poise::dispatch_event(
            framework_data,
            &ctx,
            &poise::Event::InteractionCreate { interaction },
        )
        .await;
    }
    /*
    async fn message_update(
        &self,
        ctx: serenity::Context,
        old_if_available: Option<serenity::Message>,
        new: Option<serenity::Message>,
        event: serenity::MessageUpdateEvent,
    ) {
        let shard_manager = (*self.shard_manager.lock().unwrap()).clone().unwrap();
        let framework_data = poise::FrameworkContext {
            bot_id: Default::default(),
            options: &self.options,
            user_data: &Data {},
            shard_manager: &shard_manager,
        };

        poise::dispatch_event(
            framework_data,
            &ctx,
            &poise::Event::MessageUpdate {
                old_if_available,
                new,
                event,
            },
        )
        .await;
    }
    */
    async fn voice_state_update(
        &self,
        ctx: serenity::Context,
        old: Option<serenity::VoiceState>,
        new: serenity::VoiceState,
    ) {
        handle_voice_state_update(&ctx, &new).await;
        let shard_manager = (*self.shard_manager.lock().unwrap()).clone().unwrap();
        let framework_data = poise::FrameworkContext {
            bot_id: Default::default(),
            options: &self.options,
            user_data: &Data {},
            shard_manager: &shard_manager,
        };

        poise::dispatch_event(
            framework_data,
            &ctx,
            &poise::Event::VoiceStateUpdate { old, new },
        )
        .await;
    }
}

async fn handle_voice_state_update(ctx: &serenity::Context, new: &serenity::VoiceState) {
    let Some(guild) = new
        .guild_id
        .and_then(|guild_id| guild_id.to_guild_cached(&ctx.cache)) else { return };
    let manager = songbird::get(ctx).await.unwrap().clone();
    let Some(call) = manager.get(guild.id) else { return };
    let Some(channel_id) = call.lock().await.current_channel().map(|item| item.0.into()) else { return };
    let Some(channel) = guild.channels.get(&channel_id).cloned().and_then(serenity::Channel::guild) else { return };
    if channel.members(&ctx.cache).await.unwrap().is_empty() {
        if let Err(e) = manager.remove(guild.id).await {
            log::error!("{:?}", e)
        }
    };
}
