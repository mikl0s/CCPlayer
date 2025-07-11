//! Frame buffering and queue management for smooth playback
//! 
//! Provides a thread-safe queue for buffering decoded frames with
//! PTS-based ordering and frame dropping capabilities.

use crate::renderer::VideoFrame;
use crate::utils::error::{CCPlayerError, Result};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Frame queue for buffering decoded frames
pub struct FrameQueue {
    /// Queue of frames sorted by PTS
    frames: VecDeque<VideoFrame>,
    
    /// Maximum number of frames to buffer
    max_frames: usize,
    
    /// Total size of buffered frames in bytes
    total_size: usize,
    
    /// Maximum total size in bytes (default: 100MB)
    max_size: usize,
    
    /// Statistics
    stats: QueueStats,
    
    /// Last frame PTS for ordering validation
    last_pts: Option<i64>,
}

/// Queue statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    /// Total frames added
    pub frames_added: u64,
    
    /// Total frames dropped due to queue full
    pub frames_dropped: u64,
    
    /// Total frames consumed
    pub frames_consumed: u64,
    
    /// Current queue depth
    pub current_depth: usize,
    
    /// Average queue depth
    pub avg_depth: f32,
    
    /// Maximum queue depth reached
    pub max_depth: usize,
    
    /// Total bytes processed
    pub bytes_processed: u64,
    
    /// Last update time
    pub last_update: Option<Instant>,
}

impl FrameQueue {
    /// Create a new frame queue with specified capacity
    pub fn new(max_frames: usize) -> Self {
        Self {
            frames: VecDeque::with_capacity(max_frames),
            max_frames,
            total_size: 0,
            max_size: 100 * 1024 * 1024, // 100MB default
            stats: QueueStats::default(),
            last_pts: None,
        }
    }
    
    /// Set maximum buffer size in bytes
    pub fn set_max_size(&mut self, size: usize) {
        self.max_size = size;
    }
    
    /// Push a frame to the queue
    pub fn push_frame(&mut self, frame: VideoFrame) -> Result<()> {
        // Validate PTS ordering
        if let Some(last_pts) = self.last_pts {
            if frame.pts < last_pts {
                log::warn!("Frame PTS {} is less than last PTS {}, possible ordering issue", 
                    frame.pts, last_pts);
            }
        }
        
        let frame_size = self.estimate_frame_size(&frame);
        
        // Check if we need to drop frames
        while (self.frames.len() >= self.max_frames || 
               self.total_size + frame_size > self.max_size) && 
               !self.frames.is_empty() {
            self.drop_oldest_frame();
        }
        
        // Insert frame in PTS order
        let insert_pos = self.frames.iter().position(|f| f.pts > frame.pts)
            .unwrap_or(self.frames.len());
        
        self.frames.insert(insert_pos, frame);
        self.total_size += frame_size;
        self.last_pts = Some(self.frames.back().unwrap().pts);
        
        // Update statistics
        self.stats.frames_added += 1;
        self.stats.current_depth = self.frames.len();
        self.stats.max_depth = self.stats.max_depth.max(self.frames.len());
        self.stats.bytes_processed += frame_size as u64;
        self.update_avg_depth();
        
        Ok(())
    }
    
    /// Pop the next frame from the queue
    pub fn pop_frame(&mut self) -> Option<VideoFrame> {
        if let Some(frame) = self.frames.pop_front() {
            let frame_size = self.estimate_frame_size(&frame);
            self.total_size = self.total_size.saturating_sub(frame_size);
            
            // Update statistics
            self.stats.frames_consumed += 1;
            self.stats.current_depth = self.frames.len();
            self.update_avg_depth();
            
            Some(frame)
        } else {
            None
        }
    }
    
    /// Peek at the next frame without removing it
    pub fn peek_frame(&self) -> Option<&VideoFrame> {
        self.frames.front()
    }
    
    /// Get frame at specific index without removing
    pub fn get_frame(&self, index: usize) -> Option<&VideoFrame> {
        self.frames.get(index)
    }
    
    /// Find frame closest to target PTS
    pub fn find_frame_by_pts(&self, target_pts: i64) -> Option<&VideoFrame> {
        self.frames.iter()
            .min_by_key(|frame| (frame.pts - target_pts).abs())
    }
    
    /// Drop frames older than specified PTS
    pub fn drop_frames_before(&mut self, pts: i64) {
        while let Some(frame) = self.frames.front() {
            if frame.pts < pts {
                self.drop_oldest_frame();
            } else {
                break;
            }
        }
    }
    
    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
    
    /// Get current queue length
    pub fn len(&self) -> usize {
        self.frames.len()
    }
    
    /// Check if queue is full
    pub fn is_full(&self) -> bool {
        self.frames.len() >= self.max_frames || self.total_size >= self.max_size
    }
    
    /// Clear all frames from the queue
    pub fn clear(&mut self) {
        self.frames.clear();
        self.total_size = 0;
        self.last_pts = None;
        self.stats.current_depth = 0;
    }
    
    /// Get queue statistics
    pub fn stats(&self) -> &QueueStats {
        &self.stats
    }
    
    /// Get the PTS range of buffered frames
    pub fn pts_range(&self) -> Option<(i64, i64)> {
        if self.frames.is_empty() {
            None
        } else {
            Some((
                self.frames.front().unwrap().pts,
                self.frames.back().unwrap().pts,
            ))
        }
    }
    
    /// Get buffered duration
    pub fn buffered_duration(&self) -> Duration {
        if let Some((start_pts, end_pts)) = self.pts_range() {
            Duration::from_micros((end_pts - start_pts) as u64)
        } else {
            Duration::ZERO
        }
    }
    
    /// Drop the oldest frame
    fn drop_oldest_frame(&mut self) {
        if let Some(frame) = self.frames.pop_front() {
            let frame_size = self.estimate_frame_size(&frame);
            self.total_size = self.total_size.saturating_sub(frame_size);
            self.stats.frames_dropped += 1;
            self.stats.current_depth = self.frames.len();
        }
    }
    
    /// Estimate frame size in bytes
    fn estimate_frame_size(&self, frame: &VideoFrame) -> usize {
        use crate::renderer::FrameData;
        
        match &frame.data {
            FrameData::Yuv420 { y_plane, u_plane, v_plane, .. } => {
                y_plane.len() + u_plane.len() + v_plane.len()
            }
            FrameData::Yuv422 { y_plane, u_plane, v_plane, .. } => {
                y_plane.len() + u_plane.len() + v_plane.len()
            }
            FrameData::Yuv444 { y_plane, u_plane, v_plane, .. } => {
                y_plane.len() + u_plane.len() + v_plane.len()
            }
            FrameData::Rgb { data, .. } => data.len(),
            FrameData::Rgba { data, .. } => data.len(),
            FrameData::Nv12 { y_plane, uv_plane, .. } => {
                y_plane.len() + uv_plane.len()
            }
        }
    }
    
    /// Update average queue depth
    fn update_avg_depth(&mut self) {
        let now = Instant::now();
        
        if let Some(last_update) = self.stats.last_update {
            let elapsed = now.duration_since(last_update).as_secs_f32();
            if elapsed > 0.0 {
                // Exponential moving average
                let alpha = 0.1;
                self.stats.avg_depth = self.stats.avg_depth * (1.0 - alpha) + 
                    self.stats.current_depth as f32 * alpha;
            }
        } else {
            self.stats.avg_depth = self.stats.current_depth as f32;
        }
        
        self.stats.last_update = Some(now);
    }
}

/// Frame timing controller for smooth playback
pub struct FrameTimingController {
    /// Target frame rate
    target_fps: f32,
    
    /// Frame duration in microseconds
    frame_duration: i64,
    
    /// Last presented frame time
    last_present_time: Option<Instant>,
    
    /// Last presented frame PTS
    last_pts: Option<i64>,
    
    /// Clock offset for synchronization
    clock_offset: i64,
    
    /// Playback speed multiplier
    playback_speed: f32,
    
    /// Drop frame threshold
    drop_threshold: Duration,
}

impl FrameTimingController {
    /// Create a new frame timing controller
    pub fn new(target_fps: f32) -> Self {
        let frame_duration = (1_000_000.0 / target_fps) as i64;
        
        Self {
            target_fps,
            frame_duration,
            last_present_time: None,
            last_pts: None,
            clock_offset: 0,
            playback_speed: 1.0,
            drop_threshold: Duration::from_millis(50), // Drop if more than 50ms late
        }
    }
    
    /// Check if frame should be presented now
    pub fn should_present_frame(&mut self, frame: &VideoFrame) -> FramePresentation {
        let now = Instant::now();
        
        // First frame
        if self.last_present_time.is_none() {
            self.last_present_time = Some(now);
            self.last_pts = Some(frame.pts);
            self.clock_offset = frame.pts;
            return FramePresentation::Present;
        }
        
        let elapsed = now.duration_since(self.last_present_time.unwrap());
        let elapsed_us = elapsed.as_micros() as i64;
        
        // Calculate expected PTS based on elapsed time and playback speed
        let expected_pts = self.last_pts.unwrap() + 
            (elapsed_us as f32 * self.playback_speed) as i64;
        
        // Calculate frame timing difference
        let pts_diff = frame.pts - expected_pts;
        
        if pts_diff > self.frame_duration {
            // Frame is too early
            let wait_time = Duration::from_micros((pts_diff - self.frame_duration / 2) as u64);
            FramePresentation::Wait(wait_time)
        } else if pts_diff < -self.drop_threshold.as_micros() as i64 {
            // Frame is too late, should be dropped
            FramePresentation::Drop
        } else {
            // Frame should be presented
            self.last_present_time = Some(now);
            self.last_pts = Some(frame.pts);
            FramePresentation::Present
        }
    }
    
    /// Set playback speed
    pub fn set_playback_speed(&mut self, speed: f32) {
        self.playback_speed = speed.max(0.1).min(4.0);
    }
    
    /// Reset timing state
    pub fn reset(&mut self) {
        self.last_present_time = None;
        self.last_pts = None;
        self.clock_offset = 0;
    }
    
    /// Sync to external clock
    pub fn sync_to_clock(&mut self, pts: i64) {
        self.last_pts = Some(pts);
        self.last_present_time = Some(Instant::now());
    }
}

/// Frame presentation decision
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FramePresentation {
    /// Present the frame immediately
    Present,
    
    /// Wait before presenting
    Wait(Duration),
    
    /// Drop the frame (too late)
    Drop,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::FrameData;
    
    fn create_test_frame(pts: i64) -> VideoFrame {
        VideoFrame {
            data: FrameData::Rgb {
                data: vec![0; 1920 * 1080 * 3],
                stride: 1920 * 3,
            },
            pts,
            duration: 16667, // ~60fps
            width: 1920,
            height: 1080,
            par: 1.0,
        }
    }
    
    #[test]
    fn test_frame_queue_basic() {
        let mut queue = FrameQueue::new(10);
        
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        
        // Add frames
        queue.push_frame(create_test_frame(0)).unwrap();
        queue.push_frame(create_test_frame(16667)).unwrap();
        queue.push_frame(create_test_frame(33334)).unwrap();
        
        assert_eq!(queue.len(), 3);
        assert!(!queue.is_empty());
        
        // Pop frames
        let frame1 = queue.pop_frame().unwrap();
        assert_eq!(frame1.pts, 0);
        
        let frame2 = queue.pop_frame().unwrap();
        assert_eq!(frame2.pts, 16667);
        
        assert_eq!(queue.len(), 1);
    }
    
    #[test]
    fn test_frame_queue_ordering() {
        let mut queue = FrameQueue::new(10);
        
        // Add frames out of order
        queue.push_frame(create_test_frame(33334)).unwrap();
        queue.push_frame(create_test_frame(0)).unwrap();
        queue.push_frame(create_test_frame(16667)).unwrap();
        
        // Should come out in PTS order
        assert_eq!(queue.pop_frame().unwrap().pts, 0);
        assert_eq!(queue.pop_frame().unwrap().pts, 16667);
        assert_eq!(queue.pop_frame().unwrap().pts, 33334);
    }
    
    #[test]
    fn test_frame_queue_capacity() {
        let mut queue = FrameQueue::new(3);
        
        // Fill queue
        queue.push_frame(create_test_frame(0)).unwrap();
        queue.push_frame(create_test_frame(16667)).unwrap();
        queue.push_frame(create_test_frame(33334)).unwrap();
        
        assert!(queue.is_full());
        
        // Add another frame - should drop oldest
        queue.push_frame(create_test_frame(50001)).unwrap();
        
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.stats().frames_dropped, 1);
        
        // First frame should be 16667 (0 was dropped)
        assert_eq!(queue.pop_frame().unwrap().pts, 16667);
    }
    
    #[test]
    fn test_frame_timing_controller() {
        let mut controller = FrameTimingController::new(60.0);
        
        let frame1 = create_test_frame(0);
        let frame2 = create_test_frame(16667);
        
        // First frame should always present
        assert_eq!(controller.should_present_frame(&frame1), FramePresentation::Present);
        
        // Second frame immediately after should wait
        match controller.should_present_frame(&frame2) {
            FramePresentation::Wait(duration) => {
                assert!(duration.as_millis() > 0);
            }
            _ => panic!("Expected Wait"),
        }
    }
    
    #[test]
    fn test_pts_range() {
        let mut queue = FrameQueue::new(10);
        
        assert!(queue.pts_range().is_none());
        
        queue.push_frame(create_test_frame(1000)).unwrap();
        queue.push_frame(create_test_frame(2000)).unwrap();
        queue.push_frame(create_test_frame(3000)).unwrap();
        
        let range = queue.pts_range().unwrap();
        assert_eq!(range.0, 1000);
        assert_eq!(range.1, 3000);
        
        let duration = queue.buffered_duration();
        assert_eq!(duration.as_micros(), 2000);
    }
}