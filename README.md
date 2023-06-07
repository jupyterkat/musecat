musicalcat
==========

a (mostly) pure-rust discord music bot. made for small servers

features:
- slash commands only
- everyone can command the bot, it's a free-for-all
- seeking

## building

please have the latest stable version of rust ready, instructions to install are at https://www.rust-lang.org/tools/install
in addition, this bot needs `cmake` installed to build as well

## running

please set all the necessary fields in config.toml.example and rename it to config.toml. and make sure you have both runtime dependencies and build dependencies installed

runtime dependencies:
- `yt-dlp`
- `ffmpeg`

use `cargo run --release` to build and run the bot

## installing dependencies

on windows: use `winget`, `scoop` or `chocolatey` to install `ffmpeg` and `yt-dlp`. use the visual studio installer to install `cmake`, and then run build command from the dev command prompt to get the build command to recognize it

on linux: use your distro's package manager to install it, or follow the guide on each deps
