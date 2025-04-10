use std::sync::Arc;

use poise::serenity_prelude::prelude::TypeMapKey;
use songbird::Call;
use tokio::sync::Mutex;

use crate::Context;
use crate::Error;

pub fn human_print_time(dur: std::time::Duration) -> String {
    let minutes = dur.as_secs() / 60;
    let secs = dur.as_secs() - minutes * 60;
    format!("[{:0>2}:{:0>2}]", minutes, secs)
}

pub async fn get_handler_lock(&ctx: &Context<'_>) -> Result<Option<Arc<Mutex<Call>>>, Error> {
    let Some(guild) = ctx.guild_id() else {
        ctx.say("I can only operate in a server!").await?;
        return Ok(None);
    };
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    let Some(handler_lock) = manager.get(guild) else {
        ctx.say("I'm not in voice chat yet!").await?;
        return Ok(None);
    };
    Ok(Some(handler_lock))
}

// YtDl requests need an HTTP client to operate -- we'll create and store our own.
pub struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = reqwest::Client;
}

pub struct CustomMetadata {
    pub aux_metadata: songbird::input::AuxMetadata,
    pub requested_by: String,
}
