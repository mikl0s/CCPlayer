use anyhow::Result;
use env_logger::Env;
use log::{info, error};
use clap::{Parser, ArgAction};
use std::path::PathBuf;

mod audio;
mod decoder;
mod player;
mod renderer;
mod utils;
mod window;

use player::{MediaPlayer, MediaPlayerBuilder, PlayerConfig, PlayerEvent};
use window::WindowConfig;

/// CCPlayer - A minimalist, high-performance media player
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Media file to play
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,
    
    /// Start in fullscreen mode
    #[arg(short, long)]
    fullscreen: bool,
    
    /// Set initial volume (0-100)
    #[arg(short, long, value_name = "VOLUME", default_value = "70")]
    volume: u8,
    
    /// Disable hardware acceleration
    #[arg(long = "no-hw-accel", action = ArgAction::SetFalse)]
    hardware_accel: bool,
    
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
    
    /// Window width
    #[arg(long, default_value = "1280")]
    width: u32,
    
    /// Window height
    #[arg(long, default_value = "720")]
    height: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Initialize logging
    let log_level = if args.debug { "debug" } else { "info" };
    env_logger::Builder::from_env(Env::default().default_filter_or(log_level))
        .format_timestamp_millis()
        .init();
    
    info!("Starting CCPlayer v{}", env!("CARGO_PKG_VERSION"));
    
    // Create player configuration
    let mut player_config = PlayerConfig::default();
    player_config.default_volume = (args.volume as f32) / 100.0;
    player_config.auto_play = args.file.is_some();
    
    // Create window configuration
    let mut window_config = WindowConfig::default();
    window_config.width = args.width;
    window_config.height = args.height;
    window_config.title = "CCPlayer".to_string();
    
    // Build media player
    let mut media_player = MediaPlayerBuilder::new()
        .with_config(player_config)
        .with_window_config(window_config)
        .with_hardware_acceleration(args.hardware_accel)
        .with_event_handler(Box::new(LoggingEventHandler))
        .build()?;
    
    // Subscribe to events for UI updates
    let _event_sub = media_player.subscribe_events(|event| {
        match event {
            PlayerEvent::MediaLoaded { ref info } => {
                info!("Media loaded: {} ({}x{})", 
                    info.source,
                    info.video_streams.first().map(|v| v.width).unwrap_or(0),
                    info.video_streams.first().map(|v| v.height).unwrap_or(0)
                );
            }
            PlayerEvent::PlaybackStarted => info!("Playback started"),
            PlayerEvent::PlaybackPaused => info!("Playback paused"),
            PlayerEvent::PlaybackStopped => info!("Playback stopped"),
            PlayerEvent::EndOfMedia => info!("End of media reached"),
            PlayerEvent::Error { ref message } => error!("Player error: {}", message),
            _ => {}
        }
    });
    
    // Start the player
    media_player.start()?;
    
    // Load initial file if provided
    if let Some(file_path) = args.file {
        if file_path.exists() {
            info!("Loading file: {:?}", file_path);
            match media_player.load_file(&file_path) {
                Ok(_) => {
                    if args.fullscreen {
                        media_player.set_fullscreen(true)?;
                    }
                }
                Err(e) => {
                    error!("Failed to load file: {}", e);
                    return Err(e.into());
                }
            }
        } else {
            error!("File not found: {:?}", file_path);
            return Err(anyhow::anyhow!("File not found"));
        }
    }
    
    // Keep the main thread alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        
        // Check if player is still running
        if media_player.get_state() == player::PlaybackState::Idle {
            // Could check for shutdown signal here
        }
    }
}

/// Event handler that logs events
struct LoggingEventHandler;

impl player::PlayerEventHandler for LoggingEventHandler {
    fn handle_event(&mut self, event: PlayerEvent) {
        match event {
            PlayerEvent::PositionChanged { position } => {
                // Log position changes at debug level to avoid spam
                log::debug!("Position: {:?}", position);
            }
            PlayerEvent::BufferingProgress { percent } => {
                log::debug!("Buffering: {:.1}%", percent);
            }
            PlayerEvent::VolumeChanged { volume } => {
                info!("Volume: {:.0}%", volume * 100.0);
            }
            PlayerEvent::SpeedChanged { speed } => {
                info!("Playback speed: {:.1}x", speed);
            }
            _ => {
                // Other events are handled by the main event subscription
            }
        }
    }
}