use std::time::Duration;

use crate::utils;
use crate::Context;
use crate::Error;

/// Queues a track in, keep in mind that playlists and livestreams are not supported
#[poise::command(slash_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "YouTube URL or query string"] query: String,
    #[description = "Play the track now (This will insert the track in the front of the queue and plays it!)"]
    #[flag]
    immediate: bool,
    #[description = "Loop the track"]
    #[flag]
    track_loop: bool,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say("I can only operate in a server!").await?;
        return Ok(());
    };

    let Some(channel_id) = guild_id
        .to_guild_cached(ctx.serenity_context())
        .unwrap()
        .voice_states
        .get(&ctx.author().id)
        .and_then(|vstate| vstate.channel_id)
    else {
        ctx.say("You've gotta be in a voice channel to play!")
            .await?;
        return Ok(());
    };
    ctx.defer().await?;
    let http_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<utils::HttpKey>().cloned().unwrap()
    };

    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    let call = manager.join(guild_id, channel_id).await?;
    let source = songbird::input::YoutubeDl::new(http_client, query);

    let mut source: songbird::input::Input = source.into();

    let id = format!("<@{}>", ctx.author().id);

    let aux_metadata = source.aux_metadata().await?;
    let title = aux_metadata.title.clone();

    let track = {
        let mut handler = call.lock().await;
        let track = songbird::tracks::Track::new_with_data(
            source,
            std::sync::Arc::new(utils::CustomMetadata {
                aux_metadata,
                requested_by: id,
            }),
        );

        if immediate {
            if let Some(trackhandle) = handler.queue().current() {
                trackhandle.pause()?;
                trackhandle.seek_async(Duration::from_secs(0)).await?;
            };
            handler.enqueue(track).await;
            handler.queue().modify_queue(|queue| {
                if queue.len() == 1 {
                    return;
                }
                if let Some(backelem) = queue.pop_back() {
                    queue.push_front(backelem);
                };
            });
            handler.queue().current().unwrap()
        } else {
            handler.enqueue(track).await
        }
    };

    if track_loop {
        track.enable_loop().unwrap();
    }

    ctx.say(format!(
        "Got it!. Added **{}** to the queue",
        title.as_ref().unwrap_or(&("Untitled".to_string()))
    ))
    .await?;

    Ok(())
}

/// Clears the queue, stop playing and leave the call
#[poise::command(slash_command)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say("I can only operate in a server!").await?;
        return Ok(());
    };
    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();

    match manager.remove(guild_id).await {
        Ok(()) => Ok(()),
        Err(songbird::error::JoinError::NoCall) => {
            ctx.say("I'm not playing anything!").await?;
            return Ok(());
        }
        Err(e) => Err(e),
    }?;

    ctx.say("Disconnected from server!").await?;
    Ok(())
}

/// Skips to the next track in the queue
#[poise::command(slash_command)]
pub async fn next(ctx: Context<'_>) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
        return Ok(());
    };

    if let Err(e) = handler_lock.lock().await.queue().skip() {
        ctx.say(format!("Error running command:\n```{:?}```", e))
            .await?;
        return Ok(());
    }

    ctx.say("Skipped!").await?;

    Ok(())
}

/// Shuffles all the next tracks in the queue
#[poise::command(slash_command)]
pub async fn shuffle(ctx: Context<'_>) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
        return Ok(());
    };

    let guard = handler_lock.lock().await;

    let queue = guard.queue();

    if queue.is_empty() {
        ctx.say("Nothing is queued!").await?;
        return Ok(());
    }

    queue.modify_queue(|queue| {
        use rand::seq::SliceRandom;
        let front = queue.pop_front().unwrap();
        let mut rng = rand::rng();

        queue.make_contiguous().shuffle(&mut rng);
        queue.push_front(front);
    });

    ctx.say("Shuffled!").await?;

    Ok(())
}

/// Clears all the items in the queue, except for the current item
#[poise::command(slash_command)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
        return Ok(());
    };

    let guard = handler_lock.lock().await;

    let queue = guard.queue();

    if queue.is_empty() {
        ctx.say("Nothing is queued!").await?;
        return Ok(());
    }

    if queue.len() == 1 {
        ctx.say("There's only one item in the queue, no point in clearing!")
            .await?;
        return Ok(());
    }

    queue.modify_queue(|queue| {
        let current = queue.pop_front().unwrap();
        while let Some(item) = queue.pop_back() {
            if let Err(e) = item.stop() {
                log::error!("{:?}", e);
            };
        }
        queue.push_front(current);
    });

    ctx.say("Cleared!").await?;

    Ok(())
}

/// Removes an item at an index from the track queue
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Index of the track to remove"] index: usize,
    #[description = "Number of additional tracks to remove, defaults to 0"] size: Option<usize>,
) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
        return Ok(());
    };

    let guard = handler_lock.lock().await;

    let queue = guard.queue();

    if queue.is_empty() {
        ctx.say("Nothing is queued!").await?;
        return Ok(());
    }
    let index = index.saturating_sub(1);
    let size = size.unwrap_or(0);

    queue.modify_queue(|queue| {
        let max_size = (index + size).clamp(0, queue.len());
        queue.drain(index..max_size).for_each(|track| {
            if let Err(e) = track.stop() {
                log::error!("{:?}", e);
            };
        });
    });

    ctx.say("Removed!").await?;

    Ok(())
}
