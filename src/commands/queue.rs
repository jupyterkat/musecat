use crate::utils;
use crate::Context;
use crate::Error;

use poise::serenity_prelude::colours::roles::DARK_GREEN;
use poise::serenity_prelude::colours::roles::DARK_RED;

use songbird::tracks::{LoopState, PlayMode};

fn track_title(meta: &songbird::input::Metadata) -> String {
    let source_url = meta
        .source_url
        .clone()
        .unwrap_or("https://www.youtube.com".to_string());
    let title = meta.title.clone().unwrap_or("Untitled".to_string());
    let mut title = regex::Regex::new(r"/\[.*\]/")
        .unwrap()
        .replace_all(&title, "")
        .to_string();

    if title.len() > 48 {
        title.truncate(48);
        title = format!("{title}...");
    }

    format!("[{}]({})", title, source_url)
}

fn track_duration(meta: &songbird::input::Metadata, info: songbird::tracks::TrackState) -> String {
    let position = info.position;
    let Some(duration) = meta.duration else {
        return "".to_string()
    };
    let button = {
        match info.playing {
            PlayMode::Play => "▶️",
            PlayMode::Pause => "⏸️",
            _ => "⏹️",
        }
    };

    let loop_button = {
        match info.loops {
            LoopState::Infinite => "🔁",
            LoopState::Finite(0) => "",
            LoopState::Finite(_) => "🔁",
        }
    };
    let time_slider = {
        const WIDTH: u8 = 15u8;
        let dot_pos = (WIDTH as f64 * (position.as_secs_f64() / duration.as_secs_f64())) as u8;
        (0..WIDTH)
            .map(|item| if item == dot_pos { "🔘" } else { "▬" })
            .collect::<String>()
    };
    let time = format!(
        "`{}/{}`",
        utils::human_print_time(position),
        utils::human_print_time(duration)
    );
    format!("{button}{loop_button}{time_slider}{time}")
}

fn queue_size_fmt(size: usize) -> String {
    match size {
        0usize => "-".to_string(),
        1usize => "1 song".to_string(),
        num => format!("{num} songs"),
    }
}

/// Views currently queued tracks
#[poise::command(slash_command)]
pub async fn queue(
    ctx: Context<'_>,
    #[description = "Page number"]
    #[lazy]
    page: Option<usize>,
) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
		return Ok(())
	};

    let guard = handler_lock.lock().await;

    let queue = guard.queue();

    let queue_len = queue.len();

    if queue.is_empty() {
        ctx.say("There's nothing queued!").await?;
        return Ok(());
    }

    let page = page.unwrap_or(1usize).clamp(1, usize::MAX);

    const PAGE_SIZE: usize = 10;

    let queue_pages = queue
        .current_queue()
        .chunks(PAGE_SIZE)
        .enumerate()
        .map(|(chunk_index, slice)| {
            slice
                .iter()
                .enumerate()
                .filter_map(|(index, handle)| {
                    if index == 0 && chunk_index == 0 {
                        return None;
                    }
                    let meta = handle.metadata();
                    let songnum = index + 1 + chunk_index * PAGE_SIZE;
                    let duration = meta.duration.unwrap_or(Default::default());
                    let duration = utils::human_print_time(duration);
                    let title = track_title(meta);
                    Some(format!("`{songnum}.` {title} `{duration}`\n"))
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>();

    let queue_string = queue_pages.get(page - 1).unwrap();

    let Some(trackhandle) = queue.current() else {
        ctx.say("I can't get the current track for some reason").await?;
        return Ok(());
	};

    let meta = trackhandle.metadata();
    let info = trackhandle.get_info().await?;

    ctx.send(|builder| {
        builder.embed(|embedbuilder| {
            embedbuilder
                .color(match info.playing {
                    PlayMode::Play => DARK_GREEN,
                    _ => DARK_RED,
                })
                .title(match info.playing {
                    PlayMode::Play => "Now playing",
                    PlayMode::Pause => "Paused",
                    _ => "Not playing anything now",
                })
                .description(format!(
                    "{}\nRequested by: {}\n\n{}\n\n**Up next:**\n{queue_string}",
                    track_title(meta),
                    meta.artist.as_ref().unwrap(),
                    track_duration(meta, info)
                ))
                .fields(vec![
                    ("In queue", queue_size_fmt(queue_len), true),
                    ("Page", format!("{page} out of {}", queue_pages.len()), true),
                ])
                .footer(|footer| {
                    footer.text({
                        format!(
                            "From: {}",
                            meta.channel.as_ref().unwrap_or(&"Unknown".to_string())
                        )
                    })
                })
        })
    })
    .await?;

    Ok(())
}

/// Views current tracks
#[poise::command(slash_command)]
pub async fn current(ctx: Context<'_>) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
		return Ok(())
	};

    let guard = handler_lock.lock().await;

    let Some(trackhandle) = guard.queue().current() else {
        ctx.say("Nothing is queued right now!").await?;
        return Ok(());
    };

    let meta = trackhandle.metadata();
    let info = trackhandle.get_info().await?;

    ctx.send(|builder| {
        builder.embed(|embedbuilder| {
            embedbuilder
                .color(match info.playing {
                    PlayMode::Play => DARK_GREEN,
                    _ => DARK_RED,
                })
                .title(match info.playing {
                    PlayMode::Play => "Now playing",
                    PlayMode::Pause => "Paused",
                    _ => "Not playing anything now",
                })
                .description(format!(
                    "{}\nRequested by: {}\n\n{}\n\n",
                    track_title(meta),
                    meta.artist.as_ref().unwrap(),
                    track_duration(meta, info)
                ))
                .footer(|footer| {
                    footer.text({
                        format!(
                            "From: {}",
                            meta.channel.as_ref().unwrap_or(&"Unknown".to_string())
                        )
                    })
                })
        })
    })
    .await?;

    Ok(())
}
