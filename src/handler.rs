use super::{Data, Error};
use poise::serenity_prelude::{self as serenity, ActivityData};

use super::config::get_config;

pub struct Handler {
    pub options: poise::FrameworkOptions<Data, Error>,
    pub shard_manager: std::sync::Mutex<Option<std::sync::Arc<serenity::ShardManager>>>,
}
#[serenity::async_trait]
impl serenity::EventHandler for Handler {
    async fn ready(&self, ctx: serenity::Context, data_about_bot: serenity::Ready) {
        if let Err(e) = poise::builtins::register_globally(&ctx.http, &self.options.commands).await
        {
            log::error!("{:?}", e)
        };

        let config = get_config();

        let application_id = ctx.http.application_id().unwrap_or_default();

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
            "listening" => ActivityData::listening(activity_string),
            "streaming" => {
                ActivityData::streaming(activity_string, config.bot_activity_url.as_str()).unwrap()
            }
            "playing" => ActivityData::playing(activity_string),
            "watching" => ActivityData::watching(activity_string),
            "competing" => ActivityData::competing(activity_string),
            _ => ActivityData::listening("music"),
        };

        ctx.set_presence(Some(activity), status);

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
            serenity::FullEvent::Ready { data_about_bot },
        )
        .await;
    }

    async fn guild_create(
        &self,
        ctx: serenity::Context,
        guild: serenity::Guild,
        is_new: Option<bool>,
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
            serenity::FullEvent::GuildCreate { guild, is_new },
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
            serenity::FullEvent::InteractionCreate { interaction },
        )
        .await;
    }

    async fn voice_state_update(
        &self,
        ctx: serenity::Context,
        old: Option<serenity::VoiceState>,
        new: serenity::VoiceState,
    ) {
        let config = get_config();
        if config.bot_leave_on_empty {
            if let Err(e) = handle_voice_state_update(&ctx, &new).await {
                log::error!("{:?}", e)
            };
        }
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
            serenity::FullEvent::VoiceStateUpdate { old, new },
        )
        .await;
    }
}

async fn handle_voice_state_update(
    ctx: &serenity::Context,
    new: &serenity::VoiceState,
) -> eyre::Result<()> {
    let Some(guild_id) = new.guild_id else {
        return Ok(());
    };

    let manager = songbird::get(ctx).await.unwrap().clone();

    let Some(call) = manager.get(guild_id) else {
        return Ok(());
    };

    let Some(cur_channel) = call
        .lock()
        .await
        .current_channel()
        .map(|item| item.0.into())
    else {
        return Ok(());
    };

    let channels = guild_id.channels(&ctx.http).await?;

    let Some(channel) = channels.get(&cur_channel).cloned() else {
        return Ok(());
    };

    //find non-botted users in the channel, if there isn't, then disconnect

    if channel
        .members(&ctx.cache)
        .unwrap()
        .into_iter()
        .any(|member| !member.user.bot)
    {
        if let Err(e) = manager.remove(guild_id).await {
            log::error!("{:?}", e)
        }
    };

    Ok(())
}
