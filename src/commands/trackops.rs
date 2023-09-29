use crate::utils;
use crate::Context;
use crate::Error;

/// Unpauses the current track
#[poise::command(slash_command)]
pub async fn resume(ctx: Context<'_>) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
        return Ok(());
    };

    let guard = handler_lock.lock().await;

    let Some(trackhandle) = guard.queue().current() else {
        ctx.say("Nothing is queued right now!").await?;
        return Ok(());
    };

    if let Err(e) = trackhandle.play() {
        ctx.say(format!("Error running track command:\n```{:?}```", e))
            .await?;
        return Ok(());
    }

    ctx.say("Resumed!").await?;

    Ok(())
}

/// Pauses the current track
#[poise::command(slash_command)]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
        return Ok(());
    };

    let guard = handler_lock.lock().await;

    let Some(trackhandle) = guard.queue().current() else {
        ctx.say("Nothing is queued right now!").await?;
        return Ok(());
    };

    if let Err(e) = trackhandle.pause() {
        ctx.say(format!("Error running track command:\n```{:?}```", e))
            .await?;
        return Ok(());
    };

    ctx.say("Paused!").await?;

    Ok(())
}

/// Restarts the current track
#[poise::command(slash_command)]
pub async fn replay(ctx: Context<'_>) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
        return Ok(());
    };

    let guard = handler_lock.lock().await;

    let Some(trackhandle) = guard.queue().current() else {
        ctx.say("Nothing is queued right now!").await?;
        return Ok(());
    };

    if let Err(e) = trackhandle.seek_time(std::time::Duration::from_secs(0)) {
        ctx.say(format!("Error running track command:\n```{:?}```", e))
            .await?;
        return Ok(());
    };

    ctx.say("Restarted!").await?;

    Ok(())
}

/// Restarts the current track
#[poise::command(slash_command)]
pub async fn seek(
    ctx: Context<'_>,
    #[description = "Time to seek to, E.g '1min', '2min 2s'"] time: String,
) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
        return Ok(());
    };

    let guard = handler_lock.lock().await;

    let Some(trackhandle) = guard.queue().current() else {
        ctx.say("Nothing is queued right now!").await?;
        return Ok(());
    };
    let dur = match humantime::parse_duration(time.as_str()) {
        Ok(dur) => dur,
        Err(e) => {
            ctx.say(format!("Invalid time value!\n```{:?}```", e))
                .await?;
            return Ok(());
        }
    };

    if let Some(total) = trackhandle.metadata().duration {
        if dur > total {
            ctx.say("The track is not that long!").await?;
            return Ok(());
        }
    }

    if let Err(e) = trackhandle.seek_time(dur) {
        ctx.say(format!("Error running track command:\n```{:?}```", e))
            .await?;
        return Ok(());
    };

    ctx.say(format!("Seeked to {}!", utils::human_print_time(dur)))
        .await?;

    Ok(())
}

/// Sets the current track to loop
#[poise::command(slash_command)]
pub async fn loop_current(
    ctx: Context<'_>,
    #[description = "Number of times to loop the current track, set to zero or unset for an infinite loop"]
    #[lazy]
    loop_times: Option<usize>,
) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
        return Ok(());
    };

    let guard = handler_lock.lock().await;

    let Some(trackhandle) = guard.queue().current() else {
        ctx.say("Nothing is queued right now!").await?;
        return Ok(());
    };

    if let Err(e) = match loop_times {
        Some(0usize) => trackhandle.enable_loop(),
        Some(num) => trackhandle.loop_for(num),
        None => trackhandle.enable_loop(),
    } {
        ctx.say(format!("Error running command:\n```{:?}```", e))
            .await?;
        return Ok(());
    };

    ctx.say("Current track set to loop!").await?;

    Ok(())
}

/// Stops looping the current track
#[poise::command(slash_command)]
pub async fn stop_looping(ctx: Context<'_>) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
        return Ok(());
    };

    let guard = handler_lock.lock().await;

    let Some(trackhandle) = guard.queue().current() else {
        ctx.say("Nothing is queued right now!").await?;
        return Ok(());
    };

    if let Err(e) = trackhandle.disable_loop() {
        ctx.say(format!("Error running command:\n```{:?}```", e))
            .await?;
        return Ok(());
    };

    ctx.say("Stopped looping the current track!").await?;

    Ok(())
}
