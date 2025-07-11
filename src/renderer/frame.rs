//! Frame rendering logic
//! 
//! This module handles frame presentation timing, vsync synchronization,
//! and render statistics tracking.

use crate::utils::error::Result;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Frame timing controller for smooth playback
pub struct FrameController {
    /// Target frame rate (typically 60 FPS)
    target_fps: f64,
    
    /// Frame duration in microseconds
    frame_duration: Duration,
    
    /// Last presented frame timestamp
    last_present_time: Instant,
    
    /// Frame presentation history for statistics
    frame_history: VecDeque<FrameInfo>,
    
    /// Maximum history size
    max_history: usize,
    
    /// Vsync enabled
    vsync_enabled: bool,
    
    /// Frame drop threshold (in milliseconds)
    drop_threshold: f64,
}

/// Information about a rendered frame
#[derive(Debug, Clone)]
struct FrameInfo {
    /// Frame presentation time
    present_time: Instant,
    
    /// Frame render duration
    render_duration: Duration,
    
    /// Was the frame dropped
    dropped: bool,
    
    /// Frame PTS (presentation timestamp)
    pts: i64,
}

impl FrameController {
    /// Create a new frame controller
    pub fn new(target_fps: f64, vsync_enabled: bool) -> Self {
        let frame_duration = Duration::from_secs_f64(1.0 / target_fps);
        
        Self {
            target_fps,
            frame_duration,
            last_present_time: Instant::now(),
            frame_history: VecDeque::with_capacity(240), // 4 seconds at 60 FPS
            max_history: 240,
            vsync_enabled,
            drop_threshold: 16.67 * 1.5, // 1.5x frame time for 60 FPS
        }
    }
    
    /// Check if a frame should be presented
    pub fn should_present_frame(&self, frame_pts: i64, current_time: i64) -> bool {
        // Simple check: is the frame's PTS <= current playback time?
        frame_pts <= current_time
    }
    
    /// Wait for the appropriate time to present a frame
    pub fn wait_for_present(&mut self) -> Result<()> {
        if !self.vsync_enabled {
            // Software vsync - wait until next frame time
            let elapsed = self.last_present_time.elapsed();
            if elapsed < self.frame_duration {
                std::thread::sleep(self.frame_duration - elapsed);
            }
        }
        // With vsync enabled, the GPU driver handles timing
        
        Ok(())
    }
    
    /// Record frame presentation
    pub fn record_frame_presented(&mut self, render_start: Instant, pts: i64, dropped: bool) {
        let now = Instant::now();
        let render_duration = now.duration_since(render_start);
        
        let frame_info = FrameInfo {
            present_time: now,
            render_duration,
            dropped,
            pts,
        };
        
        self.frame_history.push_back(frame_info);
        if self.frame_history.len() > self.max_history {
            self.frame_history.pop_front();
        }
        
        self.last_present_time = now;
    }
    
    /// Calculate current FPS
    pub fn calculate_fps(&self) -> f64 {
        if self.frame_history.len() < 2 {
            return 0.0;
        }
        
        // Count frames in the last second
        let one_second_ago = Instant::now() - Duration::from_secs(1);
        let recent_frames = self.frame_history
            .iter()
            .filter(|f| f.present_time > one_second_ago && !f.dropped)
            .count();
        
        recent_frames as f64
    }
    
    /// Calculate average frame time
    pub fn calculate_avg_frame_time(&self) -> f64 {
        if self.frame_history.is_empty() {
            return 0.0;
        }
        
        let recent_frames: Vec<_> = self.frame_history
            .iter()
            .rev()
            .take(60) // Last 60 frames
            .collect();
        
        if recent_frames.len() < 2 {
            return 0.0;
        }
        
        let total_duration: Duration = recent_frames
            .windows(2)
            .map(|w| w[0].present_time.duration_since(w[1].present_time))
            .sum();
        
        total_duration.as_secs_f64() * 1000.0 / (recent_frames.len() - 1) as f64
    }
    
    /// Get dropped frame count
    pub fn get_dropped_frames(&self) -> u64 {
        self.frame_history
            .iter()
            .filter(|f| f.dropped)
            .count() as u64
    }
    
    /// Check if frame rate is stable
    pub fn is_stable(&self) -> bool {
        if self.frame_history.len() < 30 {
            return false;
        }
        
        // Check variance in recent frame times
        let recent_times: Vec<f64> = self.frame_history
            .iter()
            .rev()
            .take(30)
            .zip(self.frame_history.iter().rev().skip(1).take(30))
            .map(|(f1, f2)| f1.present_time.duration_since(f2.present_time).as_secs_f64() * 1000.0)
            .collect();
        
        if recent_times.is_empty() {
            return false;
        }
        
        let mean = recent_times.iter().sum::<f64>() / recent_times.len() as f64;
        let variance = recent_times.iter()
            .map(|t| (t - mean).powi(2))
            .sum::<f64>() / recent_times.len() as f64;
        
        // Stable if standard deviation is less than 2ms
        variance.sqrt() < 2.0
    }
    
    /// Set vsync enabled state
    pub fn set_vsync(&mut self, enabled: bool) {
        self.vsync_enabled = enabled;
    }
    
    /// Set target FPS
    pub fn set_target_fps(&mut self, fps: f64) {
        self.target_fps = fps;
        self.frame_duration = Duration::from_secs_f64(1.0 / fps);
        self.drop_threshold = (1000.0 / fps) * 1.5;
    }
}

/// Frame queue for buffering decoded frames
pub struct FrameQueue<T> {
    /// Queue of frames
    frames: VecDeque<T>,
    
    /// Maximum queue size
    max_size: usize,
}

impl<T> FrameQueue<T> {
    /// Create a new frame queue
    pub fn new(max_size: usize) -> Self {
        Self {
            frames: VecDeque::with_capacity(max_size),
            max_size,
        }
    }
    
    /// Push a frame to the queue
    pub fn push(&mut self, frame: T) -> Result<()> {
        if self.frames.len() >= self.max_size {
            // Queue is full
            return Err(crate::utils::error::CCPlayerError::BufferFull(
                "Frame queue is full".to_string()
            ));
        }
        
        self.frames.push_back(frame);
        Ok(())
    }
    
    /// Pop a frame from the queue
    pub fn pop(&mut self) -> Option<T> {
        self.frames.pop_front()
    }
    
    /// Peek at the next frame without removing it
    pub fn peek(&self) -> Option<&T> {
        self.frames.front()
    }
    
    /// Get the number of frames in the queue
    pub fn len(&self) -> usize {
        self.frames.len()
    }
    
    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
    
    /// Check if the queue is full
    pub fn is_full(&self) -> bool {
        self.frames.len() >= self.max_size
    }
    
    /// Clear all frames from the queue
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_frame_controller_fps() {
        let mut controller = FrameController::new(60.0, true);
        
        // Simulate frame presentations
        let start = Instant::now();
        for i in 0..60 {
            controller.record_frame_presented(start, i * 16_667, false);
            std::thread::sleep(Duration::from_millis(16));
        }
        
        let fps = controller.calculate_fps();
        // FPS should be close to 60 (allowing for some variance)
        assert!(fps > 50.0 && fps < 70.0);
    }
    
    #[test]
    fn test_frame_queue() {
        let mut queue: FrameQueue<i32> = FrameQueue::new(3);
        
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        
        queue.push(1).unwrap();
        queue.push(2).unwrap();
        queue.push(3).unwrap();
        
        assert!(queue.is_full());
        assert!(queue.push(4).is_err());
        
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.peek(), Some(&3));
        assert_eq!(queue.pop(), Some(3));
        
        assert!(queue.is_empty());
    }
}