//! Player controller implementation for CCPlayer
//!
//! This module provides the main PlayerController that orchestrates all
//! media playback components including decoder, renderer, audio output,
//! and window management.

use crate::utils::error::{Result, CCPlayerError};
use crate::window::{Window, WindowEvent};
use crate::renderer::{Renderer, VideoFrame, Overlay, OverlayPosition, Color};
use crate::decoder::{Decoder, MediaInfo, AudioSamples};
use crate::audio::{AudioOutput, AudioFormat, AVSyncController, SyncMode, FrameAction};
use crate::player::{
    Player, PlaybackState, PlayerConfig, PlayerEvent, PlayerEventHandler,
    PlaybackStats, Playlist, RepeatMode, PlaylistItem,
};

use std::sync::{Arc, Mutex, RwLock, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::path::Path;
use std::time::{Duration, Instant};
use std::thread;
use std::collections::VecDeque;
use tokio::sync::mpsc;
use log::{info, warn, error, debug};

/// Internal player command for thread communication
#[derive(Debug, Clone)]
enum PlayerCommand {
    Load(String),
    Play,
    Pause,
    Stop,
    Seek(Duration),
    SetVolume(f32),
    SetSpeed(f32),
    SetFullscreen(bool),
    Shutdown,
}

/// Internal player state
#[derive(Debug)]
struct PlayerState {
    /// Current playback state
    state: PlaybackState,
    
    /// Current media info
    media_info: Option<MediaInfo>,
    
    /// Current position in microseconds
    position_us: i64,
    
    /// Playback speed
    speed: f32,
    
    /// Volume level (0.0 to 1.0)
    volume: f32,
    
    /// Muted state
    muted: bool,
    
    /// Fullscreen state
    fullscreen: bool,
    
    /// Playlist
    playlist: Playlist,
    
    /// Last seek position
    last_seek: Option<Duration>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            state: PlaybackState::Idle,
            media_info: None,
            position_us: 0,
            speed: 1.0,
            volume: 0.7,
            muted: false,
            fullscreen: false,
            playlist: Playlist {
                items: Vec::new(),
                current_index: None,
                shuffle: false,
                repeat_mode: RepeatMode::None,
            },
            last_seek: None,
        }
    }
}

/// Main player controller implementation
pub struct PlayerController {
    // Core components
    window: Arc<dyn Window>,
    renderer: Arc<Mutex<dyn Renderer>>,
    decoder: Arc<Mutex<dyn Decoder>>,
    audio: Arc<Mutex<dyn AudioOutput>>,
    
    // State management
    state: Arc<RwLock<PlayerState>>,
    config: PlayerConfig,
    
    // Thread management
    decoder_thread: Option<thread::JoinHandle<()>>,
    audio_thread: Option<thread::JoinHandle<()>>,
    render_thread: Option<thread::JoinHandle<()>>,
    
    // Communication channels
    command_tx: mpsc::UnboundedSender<PlayerCommand>,
    command_rx: Arc<Mutex<mpsc::UnboundedReceiver<PlayerCommand>>>,
    
    // Synchronization
    av_sync: Arc<Mutex<AVSyncController>>,
    running: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    
    // Frame queues
    video_queue: Arc<Mutex<VecDeque<VideoFrame>>>,
    audio_queue: Arc<Mutex<VecDeque<AudioSamples>>>,
    
    // Statistics
    stats: Arc<Mutex<PlaybackStats>>,
    frames_rendered: Arc<AtomicU64>,
    frames_dropped: Arc<AtomicU64>,
    
    // Event handling
    event_handlers: Arc<Mutex<Vec<Box<dyn PlayerEventHandler>>>>,
}

impl Player for PlayerController {
    fn new(
        window: Arc<dyn Window>,
        renderer: Arc<dyn Renderer>,
        decoder: Arc<dyn Decoder>,
        audio: Arc<dyn AudioOutput>,
    ) -> Result<Self> where Self: Sized {
        // Create communication channels
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        
        // Initialize A/V sync controller
        let av_sync = AVSyncController::new(SyncMode::AudioMaster);
        
        Ok(Self {
            window,
            renderer: Arc::new(Mutex::new(renderer)),
            decoder: Arc::new(Mutex::new(decoder)),
            audio: Arc::new(Mutex::new(audio)),
            state: Arc::new(RwLock::new(PlayerState::default())),
            config: PlayerConfig::default(),
            decoder_thread: None,
            audio_thread: None,
            render_thread: None,
            command_tx,
            command_rx: Arc::new(Mutex::new(command_rx)),
            av_sync: Arc::new(Mutex::new(av_sync)),
            running: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            video_queue: Arc::new(Mutex::new(VecDeque::with_capacity(30))),
            audio_queue: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            stats: Arc::new(Mutex::new(PlaybackStats::default())),
            frames_rendered: Arc::new(AtomicU64::new(0)),
            frames_dropped: Arc::new(AtomicU64::new(0)),
            event_handlers: Arc::new(Mutex::new(Vec::new())),
        })
    }
    
    fn load_file(&mut self, path: &Path) -> Result<MediaInfo> {
        info!("Loading file: {:?}", path);
        
        // Stop current playback
        self.stop()?;
        
        // Open file in decoder
        let media_info = {
            let mut decoder = self.decoder.lock().unwrap();
            decoder.open_file(path)?
        };
        
        // Update state
        {
            let mut state = self.state.write().unwrap();
            state.media_info = Some(media_info.clone());
            state.state = PlaybackState::Stopped;
            state.position_us = 0;
        }
        
        // Initialize audio format if audio stream exists
        if let Some(audio_stream) = media_info.audio_streams.first() {
            let format = AudioFormat {
                sample_rate: audio_stream.sample_rate,
                channels: audio_stream.channels as u16,
                sample_format: crate::audio::SampleFormat::F32,
                channel_layout: match audio_stream.channels {
                    1 => crate::audio::ChannelLayout::Mono,
                    2 => crate::audio::ChannelLayout::Stereo,
                    6 => crate::audio::ChannelLayout::Surround51,
                    8 => crate::audio::ChannelLayout::Surround71,
                    n => crate::audio::ChannelLayout::Custom(n as u16),
                },
            };
            
            let mut audio = self.audio.lock().unwrap();
            audio.initialize(format)?;
        }
        
        // Set video aspect ratio
        if let Some(video_stream) = media_info.video_streams.first() {
            let aspect_ratio = video_stream.width as f32 / video_stream.height as f32;
            let mut renderer = self.renderer.lock().unwrap();
            renderer.set_aspect_ratio(aspect_ratio)?;
        }
        
        // Send event
        self.send_event(PlayerEvent::MediaLoaded { info: media_info.clone() });
        
        // Auto-play if configured
        if self.config.auto_play {
            self.play()?;
        }
        
        Ok(media_info)
    }
    
    fn load_url(&mut self, url: &str) -> Result<MediaInfo> {
        info!("Loading URL: {}", url);
        
        // Stop current playback
        self.stop()?;
        
        // Open URL in decoder
        let media_info = {
            let mut decoder = self.decoder.lock().unwrap();
            decoder.open_url(url)?
        };
        
        // Update state
        {
            let mut state = self.state.write().unwrap();
            state.media_info = Some(media_info.clone());
            state.state = PlaybackState::Stopped;
            state.position_us = 0;
        }
        
        // Initialize audio format if audio stream exists
        if let Some(audio_stream) = media_info.audio_streams.first() {
            let format = AudioFormat {
                sample_rate: audio_stream.sample_rate,
                channels: audio_stream.channels as u16,
                sample_format: crate::audio::SampleFormat::F32,
                channel_layout: match audio_stream.channels {
                    1 => crate::audio::ChannelLayout::Mono,
                    2 => crate::audio::ChannelLayout::Stereo,
                    6 => crate::audio::ChannelLayout::Surround51,
                    8 => crate::audio::ChannelLayout::Surround71,
                    n => crate::audio::ChannelLayout::Custom(n as u16),
                },
            };
            
            let mut audio = self.audio.lock().unwrap();
            audio.initialize(format)?;
        }
        
        // Send event
        self.send_event(PlayerEvent::MediaLoaded { info: media_info.clone() });
        
        // Auto-play if configured
        if self.config.auto_play {
            self.play()?;
        }
        
        Ok(media_info)
    }
    
    fn play(&mut self) -> Result<()> {
        let current_state = self.state.read().unwrap().state;
        
        match current_state {
            PlaybackState::Playing => return Ok(()),
            PlaybackState::Idle => {
                return Err(CCPlayerError::InvalidInput("No media loaded".to_string()));
            }
            _ => {}
        }
        
        info!("Starting playback");
        
        // Update state
        {
            let mut state = self.state.write().unwrap();
            state.state = PlaybackState::Playing;
        }
        
        // Set flags
        self.running.store(true, Ordering::SeqCst);
        self.paused.store(false, Ordering::SeqCst);
        
        // Start playback threads
        self.start_playback_threads();
        
        // Resume audio
        {
            let mut audio = self.audio.lock().unwrap();
            audio.resume()?;
        }
        
        // Send event
        self.send_event(PlayerEvent::PlaybackStarted);
        
        Ok(())
    }
    
    fn pause(&mut self) -> Result<()> {
        if self.state.read().unwrap().state != PlaybackState::Playing {
            return Ok(());
        }
        
        info!("Pausing playback");
        
        // Update state
        {
            let mut state = self.state.write().unwrap();
            state.state = PlaybackState::Paused;
        }
        
        // Set pause flag
        self.paused.store(true, Ordering::SeqCst);
        
        // Pause audio
        {
            let mut audio = self.audio.lock().unwrap();
            audio.pause()?;
        }
        
        // Send event
        self.send_event(PlayerEvent::PlaybackPaused);
        
        Ok(())
    }
    
    fn stop(&mut self) -> Result<()> {
        info!("Stopping playback");
        
        // Update state
        {
            let mut state = self.state.write().unwrap();
            state.state = PlaybackState::Stopped;
            state.position_us = 0;
        }
        
        // Stop threads
        self.running.store(false, Ordering::SeqCst);
        
        // Stop audio
        {
            let mut audio = self.audio.lock().unwrap();
            audio.stop()?;
        }
        
        // Clear queues
        {
            let mut video_queue = self.video_queue.lock().unwrap();
            video_queue.clear();
        }
        {
            let mut audio_queue = self.audio_queue.lock().unwrap();
            audio_queue.clear();
        }
        
        // Wait for threads to finish
        if let Some(thread) = self.decoder_thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.audio_thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.render_thread.take() {
            let _ = thread.join();
        }
        
        // Send event
        self.send_event(PlayerEvent::PlaybackStopped);
        
        Ok(())
    }
    
    fn toggle_play(&mut self) -> Result<()> {
        match self.state.read().unwrap().state {
            PlaybackState::Playing => self.pause(),
            PlaybackState::Paused | PlaybackState::Stopped => self.play(),
            _ => Ok(()),
        }
    }
    
    fn seek(&mut self, position: Duration) -> Result<()> {
        info!("Seeking to {:?}", position);
        
        // Update state
        {
            let mut state = self.state.write().unwrap();
            state.last_seek = Some(position);
            state.position_us = position.as_micros() as i64;
        }
        
        // Clear queues
        {
            let mut video_queue = self.video_queue.lock().unwrap();
            video_queue.clear();
        }
        {
            let mut audio_queue = self.audio_queue.lock().unwrap();
            audio_queue.clear();
        }
        
        // Perform seek in decoder
        {
            let mut decoder = self.decoder.lock().unwrap();
            decoder.seek(position)?;
        }
        
        // Reset A/V sync
        {
            let mut sync = self.av_sync.lock().unwrap();
            sync.reset();
        }
        
        // Send event
        self.send_event(PlayerEvent::PositionChanged { position });
        
        Ok(())
    }
    
    fn seek_relative(&mut self, delta: i64) -> Result<()> {
        let current_pos = Duration::from_micros(self.state.read().unwrap().position_us as u64);
        let new_pos = if delta >= 0 {
            current_pos + Duration::from_secs(delta as u64)
        } else {
            current_pos.saturating_sub(Duration::from_secs((-delta) as u64))
        };
        
        self.seek(new_pos)
    }
    
    fn state(&self) -> PlaybackState {
        self.state.read().unwrap().state
    }
    
    fn position(&self) -> Duration {
        Duration::from_micros(self.state.read().unwrap().position_us as u64)
    }
    
    fn duration(&self) -> Duration {
        self.state.read().unwrap()
            .media_info
            .as_ref()
            .map(|info| info.duration)
            .unwrap_or_default()
    }
    
    fn set_speed(&mut self, speed: f32) -> Result<()> {
        if speed <= 0.0 || speed > 4.0 {
            return Err(CCPlayerError::InvalidInput("Speed must be between 0.0 and 4.0".to_string()));
        }
        
        {
            let mut state = self.state.write().unwrap();
            state.speed = speed;
        }
        
        // Update A/V sync
        {
            let mut sync = self.av_sync.lock().unwrap();
            sync.set_playback_speed(speed);
        }
        
        // Send event
        self.send_event(PlayerEvent::SpeedChanged { speed });
        
        Ok(())
    }
    
    fn speed(&self) -> f32 {
        self.state.read().unwrap().speed
    }
    
    fn set_volume(&mut self, volume: f32) -> Result<()> {
        let clamped_volume = volume.clamp(0.0, 1.0);
        
        {
            let mut state = self.state.write().unwrap();
            state.volume = clamped_volume;
        }
        
        // Apply volume to audio output
        {
            let mut audio = self.audio.lock().unwrap();
            audio.set_volume(clamped_volume)?;
        }
        
        // Show volume overlay
        self.show_volume_overlay(clamped_volume)?;
        
        // Send event
        self.send_event(PlayerEvent::VolumeChanged { volume: clamped_volume });
        
        Ok(())
    }
    
    fn volume(&self) -> f32 {
        self.state.read().unwrap().volume
    }
    
    fn toggle_mute(&mut self) -> Result<()> {
        let (new_muted, volume) = {
            let mut state = self.state.write().unwrap();
            state.muted = !state.muted;
            (state.muted, state.volume)
        };
        
        // Apply mute to audio output
        {
            let mut audio = self.audio.lock().unwrap();
            audio.set_volume(if new_muted { 0.0 } else { volume })?;
        }
        
        // Show volume overlay
        self.show_volume_overlay(if new_muted { 0.0 } else { volume })?;
        
        Ok(())
    }
    
    fn is_muted(&self) -> bool {
        self.state.read().unwrap().muted
    }
    
    fn set_fullscreen(&mut self, fullscreen: bool) -> Result<()> {
        {
            let mut state = self.state.write().unwrap();
            state.fullscreen = fullscreen;
        }
        
        // Apply to window
        let mut window = Arc::clone(&self.window);
        unsafe {
            let window_ptr = Arc::get_mut_unchecked(&mut window);
            window_ptr.set_fullscreen(fullscreen)?;
        }
        
        Ok(())
    }
    
    fn is_fullscreen(&self) -> bool {
        self.state.read().unwrap().fullscreen
    }
    
    fn handle_event(&mut self, event: WindowEvent) -> Result<()> {
        use crate::window::{Key, MouseButton, ControlEvent};
        
        match event {
            WindowEvent::CloseRequested => {
                self.stop()?;
                self.running.store(false, Ordering::SeqCst);
            }
            
            WindowEvent::KeyPressed { key, modifiers } => {
                match key {
                    Key::Space => self.toggle_play()?,
                    Key::Enter if modifiers.alt => self.set_fullscreen(!self.is_fullscreen())?,
                    Key::F => self.set_fullscreen(!self.is_fullscreen())?,
                    Key::M => self.toggle_mute()?,
                    Key::Left => self.seek_relative(-(self.config.seek_step as i64))?,
                    Key::Right => self.seek_relative(self.config.seek_step as i64)?,
                    Key::Up => {
                        let new_volume = self.volume() + self.config.volume_step;
                        self.set_volume(new_volume)?;
                    }
                    Key::Down => {
                        let new_volume = self.volume() - self.config.volume_step;
                        self.set_volume(new_volume)?;
                    }
                    Key::PageUp => self.seek_relative(self.config.fast_seek_step as i64)?,
                    Key::PageDown => self.seek_relative(-(self.config.fast_seek_step as i64))?,
                    Key::Minus => {
                        let new_speed = (self.speed() - 0.1).max(0.25);
                        self.set_speed(new_speed)?;
                    }
                    Key::Plus => {
                        let new_speed = (self.speed() + 0.1).min(4.0);
                        self.set_speed(new_speed)?;
                    }
                    Key::Escape if self.is_fullscreen() => self.set_fullscreen(false)?,
                    Key::Q if modifiers.ctrl => {
                        self.stop()?;
                        self.running.store(false, Ordering::SeqCst);
                    }
                    _ => {}
                }
            }
            
            WindowEvent::MouseWheel { delta } => {
                let volume_change = delta * self.config.volume_step;
                let new_volume = self.volume() + volume_change;
                self.set_volume(new_volume)?;
            }
            
            WindowEvent::ControlEvent(control) => {
                match control {
                    ControlEvent::Minimize => {
                        let mut window = Arc::clone(&self.window);
                        unsafe {
                            let window_ptr = Arc::get_mut_unchecked(&mut window);
                            window_ptr.hide()?;
                        }
                    }
                    ControlEvent::Maximize => {
                        self.set_fullscreen(!self.is_fullscreen())?;
                    }
                    ControlEvent::Close => {
                        self.stop()?;
                        self.running.store(false, Ordering::SeqCst);
                    }
                }
            }
            
            WindowEvent::FilesDropped { paths } => {
                if let Some(path) = paths.first() {
                    self.load_file(path)?;
                }
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    fn run(&mut self) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);
        
        // Main event loop
        while self.running.load(Ordering::SeqCst) {
            // Handle window events
            let mut window = Arc::clone(&self.window);
            let events = unsafe {
                let window_ptr = Arc::get_mut_unchecked(&mut window);
                window_ptr.handle_events()?
            };
            
            for event in events {
                self.handle_event(event)?;
            }
            
            // Small sleep to prevent busy waiting
            thread::sleep(Duration::from_millis(16)); // ~60 FPS event handling
        }
        
        Ok(())
    }
}

impl PlayerController {
    /// Start playback threads
    fn start_playback_threads(&mut self) {
        // Decoder thread
        if self.decoder_thread.is_none() {
            let decoder = Arc::clone(&self.decoder);
            let video_queue = Arc::clone(&self.video_queue);
            let audio_queue = Arc::clone(&self.audio_queue);
            let running = Arc::clone(&self.running);
            let paused = Arc::clone(&self.paused);
            let state = Arc::clone(&self.state);
            
            self.decoder_thread = Some(thread::spawn(move || {
                Self::decoder_thread_fn(decoder, video_queue, audio_queue, running, paused, state);
            }));
        }
        
        // Audio thread
        if self.audio_thread.is_none() {
            let audio = Arc::clone(&self.audio);
            let audio_queue = Arc::clone(&self.audio_queue);
            let av_sync = Arc::clone(&self.av_sync);
            let running = Arc::clone(&self.running);
            let paused = Arc::clone(&self.paused);
            let state = Arc::clone(&self.state);
            
            self.audio_thread = Some(thread::spawn(move || {
                Self::audio_thread_fn(audio, audio_queue, av_sync, running, paused, state);
            }));
        }
        
        // Render thread
        if self.render_thread.is_none() {
            let renderer = Arc::clone(&self.renderer);
            let video_queue = Arc::clone(&self.video_queue);
            let av_sync = Arc::clone(&self.av_sync);
            let running = Arc::clone(&self.running);
            let paused = Arc::clone(&self.paused);
            let frames_rendered = Arc::clone(&self.frames_rendered);
            let frames_dropped = Arc::clone(&self.frames_dropped);
            
            self.render_thread = Some(thread::spawn(move || {
                Self::render_thread_fn(
                    renderer,
                    video_queue,
                    av_sync,
                    running,
                    paused,
                    frames_rendered,
                    frames_dropped,
                );
            }));
        }
    }
    
    /// Decoder thread function
    fn decoder_thread_fn(
        decoder: Arc<Mutex<dyn Decoder>>,
        video_queue: Arc<Mutex<VecDeque<VideoFrame>>>,
        audio_queue: Arc<Mutex<VecDeque<AudioSamples>>>,
        running: Arc<AtomicBool>,
        paused: Arc<AtomicBool>,
        state: Arc<RwLock<PlayerState>>,
    ) {
        while running.load(Ordering::SeqCst) {
            if paused.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            
            // Check queue sizes
            let video_queue_size = video_queue.lock().unwrap().len();
            let audio_queue_size = audio_queue.lock().unwrap().len();
            
            // Don't decode too far ahead
            if video_queue_size > 25 && audio_queue_size > 90 {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            
            // Decode video frame
            if video_queue_size < 30 {
                match decoder.lock().unwrap().decode_frame() {
                    Ok(Some(frame)) => {
                        video_queue.lock().unwrap().push_back(frame);
                    }
                    Ok(None) => {
                        // End of stream
                        let mut state_guard = state.write().unwrap();
                        state_guard.state = PlaybackState::Ended;
                        break;
                    }
                    Err(e) => {
                        error!("Decoder error: {}", e);
                    }
                }
            }
            
            // Decode audio samples
            if audio_queue_size < 100 {
                match decoder.lock().unwrap().decode_audio() {
                    Ok(Some(samples)) => {
                        audio_queue.lock().unwrap().push_back(samples);
                    }
                    Ok(None) => {
                        // End of audio stream
                    }
                    Err(e) => {
                        error!("Audio decoder error: {}", e);
                    }
                }
            }
        }
    }
    
    /// Audio thread function
    fn audio_thread_fn(
        audio: Arc<Mutex<dyn AudioOutput>>,
        audio_queue: Arc<Mutex<VecDeque<AudioSamples>>>,
        av_sync: Arc<Mutex<AVSyncController>>,
        running: Arc<AtomicBool>,
        paused: Arc<AtomicBool>,
        state: Arc<RwLock<PlayerState>>,
    ) {
        while running.load(Ordering::SeqCst) {
            if paused.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            
            // Get next audio samples
            let samples = {
                let mut queue = audio_queue.lock().unwrap();
                queue.pop_front()
            };
            
            if let Some(samples) = samples {
                // Update audio clock
                {
                    let mut sync = av_sync.lock().unwrap();
                    sync.update_audio_clock(samples.pts);
                }
                
                // Update position
                {
                    let mut state_guard = state.write().unwrap();
                    state_guard.position_us = samples.pts;
                }
                
                // Play audio
                if let Err(e) = audio.lock().unwrap().play(&samples) {
                    error!("Audio playback error: {}", e);
                }
            } else {
                // No audio samples available
                thread::sleep(Duration::from_millis(5));
            }
        }
    }
    
    /// Render thread function
    fn render_thread_fn(
        renderer: Arc<Mutex<dyn Renderer>>,
        video_queue: Arc<Mutex<VecDeque<VideoFrame>>>,
        av_sync: Arc<Mutex<AVSyncController>>,
        running: Arc<AtomicBool>,
        paused: Arc<AtomicBool>,
        frames_rendered: Arc<AtomicU64>,
        frames_dropped: Arc<AtomicU64>,
    ) {
        let mut last_frame_time = Instant::now();
        let target_frame_time = Duration::from_millis(16); // ~60 FPS
        
        while running.load(Ordering::SeqCst) {
            if paused.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            
            // Get next video frame
            let frame = {
                let mut queue = video_queue.lock().unwrap();
                queue.front().cloned()
            };
            
            if let Some(frame) = frame {
                // Check A/V sync
                let action = {
                    let mut sync = av_sync.lock().unwrap();
                    sync.check_video_frame(frame.pts)
                };
                
                match action {
                    FrameAction::Display => {
                        // Render frame
                        {
                            let mut queue = video_queue.lock().unwrap();
                            queue.pop_front();
                        }
                        
                        if let Err(e) = renderer.lock().unwrap().render_frame(frame) {
                            error!("Render error: {}", e);
                        }
                        
                        if let Err(e) = renderer.lock().unwrap().present() {
                            error!("Present error: {}", e);
                        }
                        
                        frames_rendered.fetch_add(1, Ordering::SeqCst);
                    }
                    
                    FrameAction::Drop => {
                        // Drop frame to catch up
                        {
                            let mut queue = video_queue.lock().unwrap();
                            queue.pop_front();
                        }
                        frames_dropped.fetch_add(1, Ordering::SeqCst);
                        continue; // Don't wait, process next frame immediately
                    }
                    
                    FrameAction::Wait(duration) => {
                        // Wait before displaying
                        thread::sleep(duration);
                        continue;
                    }
                    
                    FrameAction::Repeat => {
                        // Display same frame again
                        if let Err(e) = renderer.lock().unwrap().present() {
                            error!("Present error: {}", e);
                        }
                    }
                }
            } else {
                // No frames available
                thread::sleep(Duration::from_millis(5));
                continue;
            }
            
            // Frame timing
            let elapsed = last_frame_time.elapsed();
            if elapsed < target_frame_time {
                thread::sleep(target_frame_time - elapsed);
            }
            last_frame_time = Instant::now();
        }
    }
    
    /// Show volume overlay
    fn show_volume_overlay(&self, volume: f32) -> Result<()> {
        let overlay = Overlay::Volume {
            level: volume,
            position: OverlayPosition::TopRight { x: 20.0, y: 20.0 },
            duration_ms: 2000,
        };
        
        self.renderer.lock().unwrap().render_overlay(overlay)?;
        Ok(())
    }
    
    /// Send event to handlers
    fn send_event(&self, event: PlayerEvent) {
        let handlers = self.event_handlers.lock().unwrap();
        for handler in handlers.iter() {
            handler.handle_event(event.clone());
        }
    }
    
    /// Add event handler
    pub fn add_event_handler(&mut self, handler: Box<dyn PlayerEventHandler>) {
        self.event_handlers.lock().unwrap().push(handler);
    }
    
    /// Get playback statistics
    pub fn get_stats(&self) -> PlaybackStats {
        let mut stats = self.stats.lock().unwrap().clone();
        stats.frames_rendered = self.frames_rendered.load(Ordering::SeqCst);
        stats.frames_dropped = self.frames_dropped.load(Ordering::SeqCst);
        stats
    }
}

impl Drop for PlayerController {
    fn drop(&mut self) {
        // Ensure cleanup
        let _ = self.stop();
    }
}