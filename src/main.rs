mod app;
mod fabric;
mod launcher;
mod paths;
mod profile;
mod skin;
mod ui;
mod versions;
mod skin_view;
mod skin_server;
mod window;
mod art;
mod browse;
mod mods;
mod packs;
mod worlds;
mod discord;
mod stats;
mod logs;
mod java;
mod settings;

use anyhow::Result;

fn main() -> Result<()> {
    if settings::load().discord {
        discord::init();
    }
    stats::init();
    let result = ui::run();
    discord::shutdown();
    result
}