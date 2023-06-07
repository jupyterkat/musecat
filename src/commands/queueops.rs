use crate::utils;
use crate::Context;
use crate::Error;

fn link_is_youtube(link: &str) -> bool {
    regex::Regex::new(r"/^(https?\:\/\/)?((www\.)?youtube\.com|youtu\.be)\/.+$/")
        .unwrap()
        .find(link)
        .is_some()
}

/// Queues a track in, keep in mind that playlists and livestreams are not supported
#[poise::command(slash_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "YouTube URL or query string"] query: String,
    #[description = "Play the track now (This will clear the queue!)"]
    #[flag]
    immediate: bool,
    #[description = "Loop the track"]
    #[flag]
    track_loop: bool,
) -> Result<(), Error> {
    let Some(guild) = ctx.guild() else {
        ctx.say("I can only operate in a server!").await?;
        return Ok(());
    };

    let Some(channel_id) =
        guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|vstate| vstate.channel_id)
    else {
        ctx.say("You've gotta be in a voice channel to play!").await?;
        return Ok(());
    };

    let manager = songbird::get(ctx.serenity_context()).await.unwrap().clone();
    let (handler_lock, res) = manager.join(guild.id, channel_id).await;
    let mut handler = handler_lock.lock().await;
    let source = match {
        if link_is_youtube(query.as_str()) {
            songbird::input::Restartable::ytdl(query, true).await
        } else {
            songbird::input::Restartable::ytdl_search(query, true).await
        }
    } {
        Ok(src) => src,
        Err(why) => {
            ctx.say(format!("Can't start the source, whoops!\n```{:?}```", why))
                .await?;
            return Ok(());
        }
    };

    let mut source: songbird::input::Input = source.into();

    let id = format!("<@{}>", ctx.author().id);

    source.metadata.as_mut().artist = Some(id);

    let track = if immediate {
        handler.queue().stop();
        handler.enqueue_source(source)
    } else {
        handler.enqueue_source(source)
    };

    if track_loop {
        track.enable_loop().unwrap();
    }

    let meta = track.metadata();

    ctx.say(format!(
        "Got it!. Added **{}** to the queue",
        meta.title.as_ref().unwrap_or(&("Untitled".to_string()))
    ))
    .await?;

    res?;
    Ok(())
}

/// Clears the queue, stop playing and leave the call
#[poise::command(slash_command)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say("I can only operate in a server!").await?;
        return Ok(())
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
		return Ok(())
	};

    let guard = handler_lock.lock().await;

    let queue = guard.queue();

    if let Err(e) = queue.skip() {
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
		return Ok(())
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
        let mut rng = rand::thread_rng();

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
		return Ok(())
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
		return Ok(())
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
