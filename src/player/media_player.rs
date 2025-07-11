//! High-level media player API for CCPlayer
//!
//! This module provides a simplified, high-level API for media playback
//! that wraps the PlayerController with additional features like
//! automatic error recovery, performance monitoring, and event dispatching.

use crate::utils::error::{Result, CCPlayerError};
use crate::window::{Window, WindowConfig, WinitWindowImpl};
use crate::renderer::{Renderer, WgpuRenderer};
use crate::decoder::{Decoder, FFmpegDecoder, MediaInfo};
use crate::audio::{AudioOutput, CpalAudioOutput};
use crate::player::{
    Player, PlayerController, PlaybackState, PlayerConfig, PlayerEvent,
    PlayerEventHandler, PlaybackStats, Playlist, PlaylistItem, RepeatMode,
};

use std::sync::{Arc, Mutex, RwLock};
use std::path::Path;
use std::time::{Duration, Instant};
use std::thread;
use tokio::runtime::Runtime;
use log::{info, warn, error};

/// Media player builder for customized configuration
pub struct MediaPlayerBuilder {
    config: PlayerConfig,
    window_config: WindowConfig,
    enable_hardware_accel: bool,
    event_handlers: Vec<Box<dyn PlayerEventHandler>>,
}

impl MediaPlayerBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            config: PlayerConfig::default(),
            window_config: WindowConfig::default(),
            enable_hardware_accel: true,
            event_handlers: Vec::new(),
        }
    }
    
    /// Set player configuration
    pub fn with_config(mut self, config: PlayerConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Set window configuration
    pub fn with_window_config(mut self, config: WindowConfig) -> Self {
        self.window_config = config;
        self
    }
    
    /// Enable or disable hardware acceleration
    pub fn with_hardware_acceleration(mut self, enabled: bool) -> Self {
        self.enable_hardware_accel = enabled;
        self
    }
    
    /// Add an event handler
    pub fn with_event_handler(mut self, handler: Box<dyn PlayerEventHandler>) -> Self {
        self.event_handlers.push(handler);
        self
    }
    
    /// Build the media player
    pub fn build(self) -> Result<MediaPlayer> {
        MediaPlayer::new_with_builder(self)
    }
}

/// High-level media player
pub struct MediaPlayer {
    /// Inner player controller
    controller: Arc<Mutex<PlayerController>>,
    
    /// Runtime for async operations
    runtime: Arc<Runtime>,
    
    /// Performance monitor
    perf_monitor: Arc<Mutex<PerformanceMonitor>>,
    
    /// Error recovery handler
    error_recovery: Arc<Mutex<ErrorRecovery>>,
    
    /// Event dispatcher
    event_dispatcher: Arc<EventDispatcher>,
    
    /// Player thread handle
    player_thread: Option<thread::JoinHandle<()>>,
}

impl MediaPlayer {
    /// Create a new media player with default settings
    pub fn new() -> Result<Self> {
        MediaPlayerBuilder::new().build()
    }
    
    /// Create a new media player with builder
    fn new_with_builder(builder: MediaPlayerBuilder) -> Result<Self> {
        info!("Initializing CCPlayer media player");
        
        // Create runtime
        let runtime = Runtime::new()
            .map_err(|e| CCPlayerError::Internal(format!("Failed to create runtime: {}", e)))?;
        
        // Create window
        let window: Arc<dyn Window> = Arc::new(
            WinitWindowImpl::new(builder.window_config)?
        );
        
        // Create renderer
        let renderer: Arc<dyn Renderer> = Arc::new(
            WgpuRenderer::new(Arc::clone(&window))?
        );
        
        // Create decoder
        let mut decoder = FFmpegDecoder::new()?;
        decoder.set_hardware_acceleration(builder.enable_hardware_accel)?;
        let decoder: Arc<dyn Decoder> = Arc::new(decoder);
        
        // Create audio output
        let audio: Arc<dyn AudioOutput> = Arc::new(
            CpalAudioOutput::new()?
        );
        
        // Create player controller
        let mut controller = PlayerController::new(
            Arc::clone(&window),
            renderer,
            decoder,
            audio,
        )?;
        
        // Add event handlers
        for handler in builder.event_handlers {
            controller.add_event_handler(handler);
        }
        
        // Create event dispatcher
        let event_dispatcher = Arc::new(EventDispatcher::new());
        controller.add_event_handler(Box::new(EventDispatcherHandler {
            dispatcher: Arc::clone(&event_dispatcher),
        }));
        
        let controller = Arc::new(Mutex::new(controller));
        
        // Create performance monitor
        let perf_monitor = Arc::new(Mutex::new(PerformanceMonitor::new()));
        
        // Create error recovery handler
        let error_recovery = Arc::new(Mutex::new(ErrorRecovery::new()));
        
        Ok(Self {
            controller,
            runtime: Arc::new(runtime),
            perf_monitor,
            error_recovery,
            event_dispatcher,
            player_thread: None,
        })
    }
    
    /// Start the media player
    pub fn start(&mut self) -> Result<()> {
        if self.player_thread.is_some() {
            return Ok(());
        }
        
        info!("Starting media player");
        
        let controller = Arc::clone(&self.controller);
        let perf_monitor = Arc::clone(&self.perf_monitor);
        
        // Start player thread
        self.player_thread = Some(thread::spawn(move || {
            // Start performance monitoring
            let monitor_handle = thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(1));
                    
                    // Update performance metrics
                    if let Ok(controller) = controller.try_lock() {
                        let stats = controller.get_stats();
                        perf_monitor.lock().unwrap().update(stats);
                    }
                }
            });
            
            // Run player event loop
            if let Ok(mut controller) = controller.lock() {
                if let Err(e) = controller.run() {
                    error!("Player error: {}", e);
                }
            }
            
            drop(monitor_handle);
        }));
        
        Ok(())
    }
    
    /// Stop the media player
    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping media player");
        
        // Stop playback
        self.controller.lock().unwrap().stop()?;
        
        // Wait for player thread
        if let Some(thread) = self.player_thread.take() {
            let _ = thread.join();
        }
        
        Ok(())
    }
    
    /// Load a media file
    pub fn load_file(&self, path: &Path) -> Result<MediaInfo> {
        info!("Loading file: {:?}", path);
        
        // Attempt to load with error recovery
        match self.controller.lock().unwrap().load_file(path) {
            Ok(info) => Ok(info),
            Err(e) => {
                error!("Failed to load file: {}", e);
                
                // Try recovery strategies
                let recovery = self.error_recovery.lock().unwrap();
                if recovery.should_retry(&e) {
                    warn!("Retrying file load after error");
                    thread::sleep(Duration::from_millis(500));
                    self.controller.lock().unwrap().load_file(path)
                } else {
                    Err(e)
                }
            }
        }
    }
    
    /// Load a media URL
    pub fn load_url(&self, url: &str) -> Result<MediaInfo> {
        info!("Loading URL: {}", url);
        
        self.controller.lock().unwrap().load_url(url)
    }
    
    /// Play the current media
    pub fn play(&self) -> Result<()> {
        self.controller.lock().unwrap().play()
    }
    
    /// Pause playback
    pub fn pause(&self) -> Result<()> {
        self.controller.lock().unwrap().pause()
    }
    
    /// Toggle play/pause
    pub fn toggle_play(&self) -> Result<()> {
        self.controller.lock().unwrap().toggle_play()
    }
    
    /// Seek to position
    pub fn seek(&self, position: Duration) -> Result<()> {
        self.controller.lock().unwrap().seek(position)
    }
    
    /// Seek relative
    pub fn seek_relative(&self, delta: i64) -> Result<()> {
        self.controller.lock().unwrap().seek_relative(delta)
    }
    
    /// Set volume (0.0 to 1.0)
    pub fn set_volume(&self, volume: f32) -> Result<()> {
        self.controller.lock().unwrap().set_volume(volume)
    }
    
    /// Get current volume
    pub fn get_volume(&self) -> f32 {
        self.controller.lock().unwrap().volume()
    }
    
    /// Toggle mute
    pub fn toggle_mute(&self) -> Result<()> {
        self.controller.lock().unwrap().toggle_mute()
    }
    
    /// Set playback speed
    pub fn set_speed(&self, speed: f32) -> Result<()> {
        self.controller.lock().unwrap().set_speed(speed)
    }
    
    /// Get playback speed
    pub fn get_speed(&self) -> f32 {
        self.controller.lock().unwrap().speed()
    }
    
    /// Set fullscreen
    pub fn set_fullscreen(&self, fullscreen: bool) -> Result<()> {
        self.controller.lock().unwrap().set_fullscreen(fullscreen)
    }
    
    /// Check if fullscreen
    pub fn is_fullscreen(&self) -> bool {
        self.controller.lock().unwrap().is_fullscreen()
    }
    
    /// Get current playback state
    pub fn get_state(&self) -> PlaybackState {
        self.controller.lock().unwrap().state()
    }
    
    /// Get current position
    pub fn get_position(&self) -> Duration {
        self.controller.lock().unwrap().position()
    }
    
    /// Get media duration
    pub fn get_duration(&self) -> Duration {
        self.controller.lock().unwrap().duration()
    }
    
    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        self.perf_monitor.lock().unwrap().get_stats()
    }
    
    /// Subscribe to events
    pub fn subscribe_events<F>(&self, callback: F) -> EventSubscription
    where
        F: Fn(PlayerEvent) + Send + Sync + 'static,
    {
        self.event_dispatcher.subscribe(callback)
    }
    
    /// Load playlist
    pub fn load_playlist(&self, items: Vec<PlaylistItem>) -> Result<()> {
        // TODO: Implement playlist management
        Ok(())
    }
    
    /// Next track
    pub fn next_track(&self) -> Result<()> {
        // TODO: Implement playlist navigation
        Ok(())
    }
    
    /// Previous track
    pub fn previous_track(&self) -> Result<()> {
        // TODO: Implement playlist navigation
        Ok(())
    }
}

/// Performance monitor
struct PerformanceMonitor {
    /// Performance history
    history: Vec<PerformanceSnapshot>,
    
    /// Start time
    start_time: Instant,
}

#[derive(Clone)]
struct PerformanceSnapshot {
    timestamp: Instant,
    stats: PlaybackStats,
}

impl PerformanceMonitor {
    fn new() -> Self {
        Self {
            history: Vec::with_capacity(3600), // 1 hour at 1 sample/sec
            start_time: Instant::now(),
        }
    }
    
    fn update(&mut self, stats: PlaybackStats) {
        let snapshot = PerformanceSnapshot {
            timestamp: Instant::now(),
            stats,
        };
        
        self.history.push(snapshot);
        
        // Keep only last hour
        let cutoff = Instant::now() - Duration::from_secs(3600);
        self.history.retain(|s| s.timestamp > cutoff);
    }
    
    fn get_stats(&self) -> PerformanceStats {
        let current = self.history.last()
            .map(|s| s.stats.clone())
            .unwrap_or_default();
        
        let fps_avg = if self.history.len() >= 60 {
            let recent = &self.history[self.history.len() - 60..];
            let total_frames: u64 = recent.iter()
                .map(|s| s.stats.frames_rendered)
                .sum();
            total_frames as f32 / 60.0
        } else {
            0.0
        };
        
        PerformanceStats {
            current,
            average_fps: fps_avg,
            uptime: self.start_time.elapsed(),
            total_frames: self.history.last()
                .map(|s| s.stats.frames_rendered)
                .unwrap_or(0),
            total_drops: self.history.last()
                .map(|s| s.stats.frames_dropped)
                .unwrap_or(0),
        }
    }
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// Current stats
    pub current: PlaybackStats,
    
    /// Average FPS over last minute
    pub average_fps: f32,
    
    /// Player uptime
    pub uptime: Duration,
    
    /// Total frames rendered
    pub total_frames: u64,
    
    /// Total frames dropped
    pub total_drops: u64,
}

/// Error recovery handler
struct ErrorRecovery {
    /// Error history
    error_history: Vec<(Instant, CCPlayerError)>,
    
    /// Recovery strategies
    strategies: Vec<RecoveryStrategy>,
}

#[derive(Debug)]
enum RecoveryStrategy {
    Retry { max_attempts: u32, delay_ms: u64 },
    Reinitialize,
    Fallback,
}

impl ErrorRecovery {
    fn new() -> Self {
        Self {
            error_history: Vec::new(),
            strategies: vec![
                RecoveryStrategy::Retry { max_attempts: 3, delay_ms: 500 },
            ],
        }
    }
    
    fn should_retry(&self, error: &CCPlayerError) -> bool {
        // Simple retry logic - can be expanded
        matches!(error, 
            CCPlayerError::Decoder(_) | 
            CCPlayerError::Audio(_) |
            CCPlayerError::FileIO(_)
        )
    }
}

/// Event dispatcher
struct EventDispatcher {
    subscribers: Arc<RwLock<Vec<Box<dyn Fn(PlayerEvent) + Send + Sync>>>>,
}

impl EventDispatcher {
    fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    fn subscribe<F>(&self, callback: F) -> EventSubscription
    where
        F: Fn(PlayerEvent) + Send + Sync + 'static,
    {
        let mut subs = self.subscribers.write().unwrap();
        let id = subs.len();
        subs.push(Box::new(callback));
        
        EventSubscription {
            id,
            dispatcher: Arc::clone(&self.subscribers),
        }
    }
    
    fn dispatch(&self, event: PlayerEvent) {
        let subs = self.subscribers.read().unwrap();
        for callback in subs.iter() {
            callback(event.clone());
        }
    }
}

/// Event dispatcher handler for PlayerController
struct EventDispatcherHandler {
    dispatcher: Arc<EventDispatcher>,
}

impl PlayerEventHandler for EventDispatcherHandler {
    fn handle_event(&mut self, event: PlayerEvent) {
        self.dispatcher.dispatch(event);
    }
}

/// Event subscription handle
pub struct EventSubscription {
    id: usize,
    dispatcher: Arc<RwLock<Vec<Box<dyn Fn(PlayerEvent) + Send + Sync>>>>,
}

impl Drop for EventSubscription {
    fn drop(&mut self) {
        // In a real implementation, would remove the specific subscriber
        // For now, we'll leave them in place
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_media_player_builder() {
        let builder = MediaPlayerBuilder::new()
            .with_hardware_acceleration(false);
        
        assert!(!builder.enable_hardware_accel);
    }
    
    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();
        
        let stats = PlaybackStats {
            frames_rendered: 100,
            frames_dropped: 1,
            ..Default::default()
        };
        
        monitor.update(stats.clone());
        
        let perf_stats = monitor.get_stats();
        assert_eq!(perf_stats.current.frames_rendered, 100);
        assert_eq!(perf_stats.total_frames, 100);
    }
    
    #[test]
    fn test_error_recovery() {
        let recovery = ErrorRecovery::new();
        
        let decoder_error = CCPlayerError::Decoder("Test error".to_string());
        assert!(recovery.should_retry(&decoder_error));
        
        let window_error = CCPlayerError::Window("Test error".to_string());
        assert!(!recovery.should_retry(&window_error));
    }
}