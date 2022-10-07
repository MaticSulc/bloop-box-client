extern crate core;

use std::path::Path;

use anyhow::{anyhow, Result};
use env_logger::{Builder, Env};
use tokio::fs;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio_graceful_shutdown::{IntoSubsystem, Toplevel};

use crate::subsystems::audio_player::AudioPlayer;
use crate::subsystems::config_manager::ConfigManager;
use crate::subsystems::controller::Controller;
use crate::subsystems::led::Led;
use crate::subsystems::networker::Networker;
use crate::subsystems::volume_control::VolumeControl;

mod nfc;
mod subsystems;
mod wifi;

#[tokio::main]
async fn main() -> Result<()> {
    Builder::from_env(Env::default().default_filter_or("debug")).init();

    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Home directory not found"))?;
    let local_dir = Path::new(&home_dir).join(".boop-box");
    let cache_dir = Path::new(&local_dir).join("cache");

    if !local_dir.is_dir() {
        fs::create_dir(&local_dir).await?;
    }

    if !cache_dir.is_dir() {
        fs::create_dir(&cache_dir).await?;
    }

    let (audio_player_tx, audio_player_rx) = mpsc::channel(8);
    let (led_tx, led_rx) = mpsc::channel(8);
    let (config_tx, config_rx) = mpsc::channel(8);
    let (networker_tx, networker_rx) = mpsc::channel(8);
    let (networker_status_tx, networker_status_rx) = mpsc::channel(8);

    Toplevel::new()
        .start("Led", Led::new(led_rx).into_subsystem())
        .start(
            "ConfigManager",
            ConfigManager::new(&local_dir, config_rx).into_subsystem(),
        )
        .start(
            "AudioPlayer",
            AudioPlayer::new(cache_dir.clone(), config_tx.clone(), audio_player_rx)
                .into_subsystem(),
        )
        .start(
            "VolumeControl",
            VolumeControl::new(audio_player_tx.clone()).into_subsystem(),
        )
        .start(
            "Networker",
            Networker::new(networker_rx, networker_status_tx, config_tx.clone()).into_subsystem(),
        )
        .start(
            "Controller",
            Controller::new(
                cache_dir,
                audio_player_tx,
                led_tx,
                config_tx,
                networker_tx,
                networker_status_rx,
            )
            .into_subsystem(),
        )
        .catch_signals()
        .handle_shutdown_requests(Duration::from_millis(1000))
        .await
}
