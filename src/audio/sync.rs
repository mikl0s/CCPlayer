//! Audio/video synchronization logic for CCPlayer
//! 
//! This module handles precise synchronization between audio and video streams
//! using presentation timestamps (PTS) and clock management.

use crate::utils::error::{CCPlayerError, Result};
use parking_lot::{Mutex, RwLock};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Maximum allowed sync deviation before correction (40ms)
const MAX_SYNC_DEVIATION_US: i64 = 40_000;

/// Target sync accuracy (1ms)
const SYNC_TARGET_US: i64 = 1_000;

/// Sync adjustment speed (samples per second)
const SYNC_ADJUSTMENT_RATE: f32 = 0.001;

/// Audio/Video synchronization controller
pub struct AVSyncController {
    /// Master clock (usually audio)
    master_clock: Arc<MasterClock>,
    
    /// Video clock
    video_clock: Arc<VideoClock>,
    
    /// Sync statistics
    stats: Arc<RwLock<SyncStats>>,
    
    /// Sync mode
    mode: Arc<RwLock<SyncMode>>,
    
    /// Sync correction enabled
    correction_enabled: Arc<AtomicBool>,
}

/// Synchronization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// Audio is master clock (default)
    AudioMaster,
    
    /// Video is master clock
    VideoMaster,
    
    /// External clock (e.g., system time)
    ExternalClock,
    
    /// Free running (no sync)
    FreeRunning,
}

/// Master clock for synchronization
pub struct MasterClock {
    /// Current PTS in microseconds
    current_pts: AtomicI64,
    
    /// Clock start time
    start_time: Mutex<Option<Instant>>,
    
    /// Total pause duration
    pause_duration: Mutex<Duration>,
    
    /// Clock speed adjustment (1.0 = normal speed)
    speed: Mutex<f32>,
    
    /// External time source (for ExternalClock mode)
    external_source: Mutex<Option<Box<dyn ExternalTimeSource>>>,
}

/// Video clock
pub struct VideoClock {
    /// Last displayed frame PTS
    last_pts: AtomicI64,
    
    /// Next frame PTS
    next_pts: AtomicI64,
    
    /// Frame duration in microseconds
    frame_duration: AtomicI64,
    
    /// Dropped frames counter
    dropped_frames: AtomicI64,
    
    /// Repeated frames counter
    repeated_frames: AtomicI64,
}

/// Synchronization statistics
#[derive(Debug, Default)]
pub struct SyncStats {
    /// Current sync error in microseconds
    pub sync_error: i64,
    
    /// Average sync error
    pub avg_sync_error: f64,
    
    /// Maximum sync error observed
    pub max_sync_error: i64,
    
    /// Number of sync corrections
    pub corrections: u64,
    
    /// Number of dropped video frames
    pub dropped_frames: u64,
    
    /// Number of repeated video frames
    pub repeated_frames: u64,
    
    /// Audio buffer underruns
    pub audio_underruns: u64,
}

/// External time source trait for custom clock implementations
pub trait ExternalTimeSource: Send + Sync {
    /// Get current time in microseconds
    fn get_time_us(&self) -> i64;
    
    /// Get clock speed (1.0 = normal)
    fn get_speed(&self) -> f32 {
        1.0
    }
}

impl AVSyncController {
    /// Create a new AV sync controller
    pub fn new() -> Self {
        Self {
            master_clock: Arc::new(MasterClock::new()),
            video_clock: Arc::new(VideoClock::new()),
            stats: Arc::new(RwLock::new(SyncStats::default())),
            mode: Arc::new(RwLock::new(SyncMode::AudioMaster)),
            correction_enabled: Arc::new(AtomicBool::new(true)),
        }
    }
    
    /// Update audio clock
    pub fn update_audio(&self, pts: i64, _samples: u64) {
        match *self.mode.read() {
            SyncMode::AudioMaster => {
                self.master_clock.set_pts(pts);
            }
            _ => {
                // Audio follows master clock
                let master_pts = self.master_clock.get_pts();
                let error = pts - master_pts;
                self.update_sync_stats(error);
            }
        }
    }
    
    /// Check if video frame should be displayed
    pub fn should_display_frame(&self, frame_pts: i64) -> FrameAction {
        let master_pts = self.master_clock.get_pts();
        let video_clock = &self.video_clock;
        
        // Calculate sync error
        let sync_error = frame_pts - master_pts;
        
        // Update statistics
        self.update_sync_stats(sync_error);
        
        // Determine action based on sync error
        if sync_error < -MAX_SYNC_DEVIATION_US {
            // Frame is too late, drop it
            video_clock.dropped_frames.fetch_add(1, Ordering::Relaxed);
            self.stats.write().dropped_frames += 1;
            FrameAction::Drop
        } else if sync_error > MAX_SYNC_DEVIATION_US {
            // Frame is too early, wait
            FrameAction::Wait(Duration::from_micros(sync_error as u64))
        } else if sync_error.abs() < SYNC_TARGET_US {
            // Frame is in sync, display it
            video_clock.last_pts.store(frame_pts, Ordering::Relaxed);
            FrameAction::Display
        } else {
            // Small sync error, display with timing adjustment
            video_clock.last_pts.store(frame_pts, Ordering::Relaxed);
            FrameAction::DisplayWithAdjustment(sync_error)
        }
    }
    
    /// Update sync statistics
    fn update_sync_stats(&self, error: i64) {
        let mut stats = self.stats.write();
        stats.sync_error = error;
        
        // Update average (exponential moving average)
        const ALPHA: f64 = 0.1;
        stats.avg_sync_error = stats.avg_sync_error * (1.0 - ALPHA) + error as f64 * ALPHA;
        
        // Update max error
        stats.max_sync_error = stats.max_sync_error.max(error.abs());
    }
    
    /// Set synchronization mode
    pub fn set_mode(&self, mode: SyncMode) {
        *self.mode.write() = mode;
    }
    
    /// Get current synchronization mode
    pub fn get_mode(&self) -> SyncMode {
        *self.mode.read()
    }
    
    /// Enable or disable sync correction
    pub fn set_correction_enabled(&self, enabled: bool) {
        self.correction_enabled.store(enabled, Ordering::Relaxed);
    }
    
    /// Get sync statistics
    pub fn get_stats(&self) -> SyncStats {
        let stats = self.stats.read();
        SyncStats {
            sync_error: stats.sync_error,
            avg_sync_error: stats.avg_sync_error,
            max_sync_error: stats.max_sync_error,
            corrections: stats.corrections,
            dropped_frames: self.video_clock.dropped_frames.load(Ordering::Relaxed) as u64,
            repeated_frames: self.video_clock.repeated_frames.load(Ordering::Relaxed) as u64,
            audio_underruns: stats.audio_underruns,
        }
    }
    
    /// Reset sync statistics
    pub fn reset_stats(&self) {
        *self.stats.write() = SyncStats::default();
        self.video_clock.dropped_frames.store(0, Ordering::Relaxed);
        self.video_clock.repeated_frames.store(0, Ordering::Relaxed);
    }
    
    /// Set frame duration for video clock
    pub fn set_frame_duration(&self, duration_us: i64) {
        self.video_clock.frame_duration.store(duration_us, Ordering::Relaxed);
    }
    
    /// Get master clock
    pub fn master_clock(&self) -> &Arc<MasterClock> {
        &self.master_clock
    }
    
    /// Get video clock
    pub fn video_clock(&self) -> &Arc<VideoClock> {
        &self.video_clock
    }
}

impl MasterClock {
    fn new() -> Self {
        Self {
            current_pts: AtomicI64::new(0),
            start_time: Mutex::new(None),
            pause_duration: Mutex::new(Duration::ZERO),
            speed: Mutex::new(1.0),
            external_source: Mutex::new(None),
        }
    }
    
    /// Set current PTS
    pub fn set_pts(&self, pts: i64) {
        self.current_pts.store(pts, Ordering::Relaxed);
    }
    
    /// Get current PTS
    pub fn get_pts(&self) -> i64 {
        self.current_pts.load(Ordering::Relaxed)
    }
    
    /// Start the clock
    pub fn start(&self) {
        *self.start_time.lock() = Some(Instant::now());
    }
    
    /// Pause the clock
    pub fn pause(&self) {
        // Implementation depends on specific requirements
    }
    
    /// Resume the clock
    pub fn resume(&self) {
        // Implementation depends on specific requirements
    }
    
    /// Set playback speed
    pub fn set_speed(&self, speed: f32) {
        *self.speed.lock() = speed.max(0.1).min(4.0); // Clamp to reasonable range
    }
    
    /// Get playback speed
    pub fn get_speed(&self) -> f32 {
        *self.speed.lock()
    }
    
    /// Set external time source
    pub fn set_external_source(&self, source: Box<dyn ExternalTimeSource>) {
        *self.external_source.lock() = Some(source);
    }
    
    /// Calculate real time elapsed since start
    pub fn get_real_time(&self) -> Option<Duration> {
        self.start_time.lock().map(|start| {
            let elapsed = start.elapsed();
            let pause_duration = *self.pause_duration.lock();
            elapsed.saturating_sub(pause_duration)
        })
    }
}

impl VideoClock {
    fn new() -> Self {
        Self {
            last_pts: AtomicI64::new(0),
            next_pts: AtomicI64::new(0),
            frame_duration: AtomicI64::new(16_667), // Default to 60 FPS
            dropped_frames: AtomicI64::new(0),
            repeated_frames: AtomicI64::new(0),
        }
    }
    
    /// Get last displayed frame PTS
    pub fn get_last_pts(&self) -> i64 {
        self.last_pts.load(Ordering::Relaxed)
    }
    
    /// Set next frame PTS
    pub fn set_next_pts(&self, pts: i64) {
        self.next_pts.store(pts, Ordering::Relaxed);
    }
    
    /// Get next frame PTS
    pub fn get_next_pts(&self) -> i64 {
        self.next_pts.load(Ordering::Relaxed)
    }
    
    /// Get frame duration
    pub fn get_frame_duration(&self) -> i64 {
        self.frame_duration.load(Ordering::Relaxed)
    }
}

/// Action to take for a video frame
#[derive(Debug, Clone)]
pub enum FrameAction {
    /// Display the frame immediately
    Display,
    
    /// Display with timing adjustment (microseconds)
    DisplayWithAdjustment(i64),
    
    /// Wait before displaying (duration)
    Wait(Duration),
    
    /// Drop the frame (too late)
    Drop,
    
    /// Repeat previous frame
    Repeat,
}

/// Sync adjustment calculator for smooth corrections
pub struct SyncAdjuster {
    /// Target sync error (usually 0)
    target: i64,
    
    /// Current adjustment in microseconds
    current_adjustment: i64,
    
    /// Adjustment speed
    speed: f32,
}

impl SyncAdjuster {
    /// Create a new sync adjuster
    pub fn new() -> Self {
        Self {
            target: 0,
            current_adjustment: 0,
            speed: SYNC_ADJUSTMENT_RATE,
        }
    }
    
    /// Calculate adjustment for given error
    pub fn calculate_adjustment(&mut self, error: i64) -> i64 {
        // Simple proportional controller
        let adjustment = (error as f32 * self.speed) as i64;
        self.current_adjustment = adjustment;
        adjustment
    }
    
    /// Reset adjuster
    pub fn reset(&mut self) {
        self.current_adjustment = 0;
    }
    
    /// Set adjustment speed
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.0001, 0.1);
    }
}

/// System time source for external clock mode
pub struct SystemTimeSource {
    start_time: Instant,
}

impl SystemTimeSource {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

impl ExternalTimeSource for SystemTimeSource {
    fn get_time_us(&self) -> i64 {
        self.start_time.elapsed().as_micros() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sync_controller_creation() {
        let sync = AVSyncController::new();
        assert_eq!(sync.get_mode(), SyncMode::AudioMaster);
        assert_eq!(sync.master_clock.get_pts(), 0);
    }
    
    #[test]
    fn test_frame_action_decision() {
        let sync = AVSyncController::new();
        sync.master_clock.set_pts(1_000_000); // 1 second
        
        // Frame too late
        let action = sync.should_display_frame(950_000);
        assert!(matches!(action, FrameAction::Drop));
        
        // Frame in sync
        let action = sync.should_display_frame(1_000_500);
        assert!(matches!(action, FrameAction::Display));
        
        // Frame too early
        let action = sync.should_display_frame(1_100_000);
        assert!(matches!(action, FrameAction::Wait(_)));
    }
    
    #[test]
    fn test_sync_stats() {
        let sync = AVSyncController::new();
        
        // Update with some errors
        sync.update_sync_stats(5_000);
        sync.update_sync_stats(-3_000);
        sync.update_sync_stats(10_000);
        
        let stats = sync.get_stats();
        assert_eq!(stats.max_sync_error, 10_000);
        assert!(stats.avg_sync_error != 0.0);
    }
    
    #[test]
    fn test_master_clock_speed() {
        let clock = MasterClock::new();
        
        clock.set_speed(2.0);
        assert_eq!(clock.get_speed(), 2.0);
        
        clock.set_speed(0.5);
        assert_eq!(clock.get_speed(), 0.5);
        
        // Test clamping
        clock.set_speed(10.0);
        assert_eq!(clock.get_speed(), 4.0);
    }
    
    #[test]
    fn test_sync_adjuster() {
        let mut adjuster = SyncAdjuster::new();
        
        let adjustment = adjuster.calculate_adjustment(10_000);
        assert_eq!(adjustment, 10); // 10_000 * 0.001
        
        adjuster.set_speed(0.01);
        let adjustment = adjuster.calculate_adjustment(10_000);
        assert_eq!(adjustment, 100); // 10_000 * 0.01
    }
}