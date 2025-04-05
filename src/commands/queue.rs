use crate::utils;
use crate::utils::CustomMetadata;
use crate::Context;
use crate::Error;

use std::fmt::Write;

use poise::serenity_prelude::colours::roles::DARK_GREEN;
use poise::serenity_prelude::colours::roles::DARK_RED;

use songbird::tracks::{LoopState, PlayMode};

fn track_title(url: Option<String>, title: Option<String>) -> String {
    let source_url = url.unwrap_or("https://www.youtube.com".to_string());
    let title = title.unwrap_or("Untitled".to_string());
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

fn track_duration(
    duration: Option<std::time::Duration>,
    info: songbird::tracks::TrackState,
) -> String {
    let position = info.position;
    let Some(duration) = duration else {
        return "".to_string();
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
        return Ok(());
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
                .filter(|(index, _)| !(*index == 0 && chunk_index == 0))
                .fold("".to_owned(), |mut string, (index, handle)| {
                    let (url, duration, title) = (
                        handle
                            .data::<CustomMetadata>()
                            .aux_metadata
                            .source_url
                            .clone(),
                        handle.data::<CustomMetadata>().aux_metadata.duration,
                        handle.data::<CustomMetadata>().aux_metadata.title.clone(),
                    );

                    let songnum = index + 1 + chunk_index * PAGE_SIZE;
                    let duration = duration.unwrap_or_else(Default::default);
                    let duration = utils::human_print_time(duration);
                    let title = track_title(url, title);
                    _ = writeln!(&mut string, "`{songnum}.` {title} `{duration}`");
                    string
                })
        })
        .collect::<Vec<_>>();

    let queue_string = queue_pages.get(page - 1).unwrap();

    let Some(trackhandle) = queue.current() else {
        ctx.say("I can't get the current track for some reason")
            .await?;
        return Ok(());
    };

    let (url, duration, title, requested_by, channel) = (
        trackhandle
            .data::<CustomMetadata>()
            .aux_metadata
            .source_url
            .clone(),
        trackhandle.data::<CustomMetadata>().aux_metadata.duration,
        trackhandle
            .data::<CustomMetadata>()
            .aux_metadata
            .title
            .clone(),
        trackhandle.data::<CustomMetadata>().requested_by.clone(),
        trackhandle
            .data::<CustomMetadata>()
            .aux_metadata
            .channel
            .clone(),
    );
    let info = trackhandle.get_info().await?;
    ctx.send(
        poise::CreateReply::default().embed(
            poise::serenity_prelude::CreateEmbed::default()
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
                    "{}\nRequested by: {requested_by}\n\n{}\n\n**Up next:**\n{queue_string}",
                    track_title(url, title),
                    track_duration(duration, info)
                ))
                .fields(vec![
                    ("In queue", queue_size_fmt(queue_len), true),
                    ("Page", format!("{page} out of {}", queue_pages.len()), true),
                ])
                .footer(poise::serenity_prelude::CreateEmbedFooter::new(format!(
                    "From: {}",
                    channel.unwrap_or("Unknown".to_string())
                ))),
        ),
    )
    .await?;

    Ok(())
}

/// Views current tracks
#[poise::command(slash_command)]
pub async fn current(ctx: Context<'_>) -> Result<(), Error> {
    let Some(handler_lock) = utils::get_handler_lock(&ctx).await? else {
        return Ok(());
    };

    let Some(trackhandle) = handler_lock.lock().await.queue().current() else {
        ctx.say("Nothing is queued right now!").await?;
        return Ok(());
    };

    let (url, duration, title, requested_by, channel) = (
        trackhandle
            .data::<CustomMetadata>()
            .aux_metadata
            .source_url
            .clone(),
        trackhandle.data::<CustomMetadata>().aux_metadata.duration,
        trackhandle
            .data::<CustomMetadata>()
            .aux_metadata
            .title
            .clone(),
        trackhandle.data::<CustomMetadata>().requested_by.clone(),
        trackhandle
            .data::<CustomMetadata>()
            .aux_metadata
            .channel
            .clone(),
    );

    let info = trackhandle.get_info().await?;

    ctx.send(
        poise::CreateReply::default().embed(
            poise::serenity_prelude::CreateEmbed::default()
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
                    track_title(url, title),
                    requested_by,
                    track_duration(duration, info)
                ))
                .footer(poise::serenity_prelude::CreateEmbedFooter::new(format!(
                    "From: {}",
                    channel.unwrap_or("Unknown".to_string())
                ))),
        ),
    )
    .await?;

    Ok(())
}
