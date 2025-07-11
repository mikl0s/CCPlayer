//! Integration test utilities for CCPlayer
//!
//! This module provides common utilities for integration testing including:
//! - Test media file generation
//! - Mock implementations
//! - Test fixtures and helpers

use anyhow::Result;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test fixture for integration tests
pub struct TestFixture {
    pub temp_dir: TempDir,
    pub media_files: MediaFiles,
}

/// Collection of test media files
pub struct MediaFiles {
    pub video_h264: PathBuf,
    pub video_h265: PathBuf,
    pub audio_mp3: PathBuf,
    pub audio_aac: PathBuf,
    pub image_jpg: PathBuf,
}

impl TestFixture {
    /// Create a new test fixture with generated media files
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let media_files = MediaFiles::generate(&temp_dir)?;
        
        Ok(Self {
            temp_dir,
            media_files,
        })
    }
    
    /// Get the path to the temporary directory
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }
}

impl MediaFiles {
    /// Generate test media files in the given directory
    fn generate(dir: &TempDir) -> Result<Self> {
        // Generate a simple test video file (1 second, 30fps, 320x240)
        let video_h264 = Self::generate_video_h264(dir)?;
        let video_h265 = Self::generate_video_h265(dir)?;
        
        // Generate test audio files
        let audio_mp3 = Self::generate_audio_mp3(dir)?;
        let audio_aac = Self::generate_audio_aac(dir)?;
        
        // Generate a test image
        let image_jpg = Self::generate_image_jpg(dir)?;
        
        Ok(Self {
            video_h264,
            video_h265,
            audio_mp3,
            audio_aac,
            image_jpg,
        })
    }
    
    fn generate_video_h264(dir: &TempDir) -> Result<PathBuf> {
        // Generate a simple H.264 video file using a test pattern
        // In a real implementation, this would use FFmpeg to create a proper video
        let path = dir.path().join("test_video_h264.mp4");
        
        // For now, create a minimal MP4 file structure
        // This is a placeholder - real implementation would use FFmpeg
        std::fs::write(&path, b"fake_h264_video_data")?;
        
        Ok(path)
    }
    
    fn generate_video_h265(dir: &TempDir) -> Result<PathBuf> {
        let path = dir.path().join("test_video_h265.mp4");
        std::fs::write(&path, b"fake_h265_video_data")?;
        Ok(path)
    }
    
    fn generate_audio_mp3(dir: &TempDir) -> Result<PathBuf> {
        let path = dir.path().join("test_audio.mp3");
        
        // Generate a simple sine wave audio file
        // For testing purposes, we'll create a minimal MP3 header
        std::fs::write(&path, b"fake_mp3_audio_data")?;
        
        Ok(path)
    }
    
    fn generate_audio_aac(dir: &TempDir) -> Result<PathBuf> {
        let path = dir.path().join("test_audio.aac");
        std::fs::write(&path, b"fake_aac_audio_data")?;
        Ok(path)
    }
    
    fn generate_image_jpg(dir: &TempDir) -> Result<PathBuf> {
        use image::{ImageBuffer, Rgb};
        
        let path = dir.path().join("test_image.jpg");
        
        // Create a simple 320x240 test pattern image
        let img = ImageBuffer::from_fn(320, 240, |x, y| {
            let r = (x * 255 / 320) as u8;
            let g = (y * 255 / 240) as u8;
            let b = 128;
            Rgb([r, g, b])
        });
        
        img.save(&path)?;
        Ok(path)
    }
}

/// Mock window event generator for testing
pub mod mock_events {
    use ccplayer::window::events::WindowEvent;
    use std::time::Duration;
    
    /// Generate a sequence of window events for testing
    pub fn generate_event_sequence() -> Vec<WindowEvent> {
        vec![
            WindowEvent::Resized { width: 1280, height: 720 },
            WindowEvent::MouseInput {
                button: winit::event::MouseButton::Left,
                state: winit::event::ElementState::Pressed,
            },
            WindowEvent::MouseMoved { x: 100.0, y: 100.0 },
            WindowEvent::MouseInput {
                button: winit::event::MouseButton::Left,
                state: winit::event::ElementState::Released,
            },
            WindowEvent::KeyboardInput {
                key: winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space),
                state: winit::event::ElementState::Pressed,
            },
        ]
    }
    
    /// Generate a drag event sequence
    pub fn generate_drag_sequence() -> Vec<WindowEvent> {
        vec![
            WindowEvent::MouseInput {
                button: winit::event::MouseButton::Left,
                state: winit::event::ElementState::Pressed,
            },
            WindowEvent::MouseMoved { x: 100.0, y: 100.0 },
            WindowEvent::MouseMoved { x: 150.0, y: 120.0 },
            WindowEvent::MouseMoved { x: 200.0, y: 140.0 },
            WindowEvent::MouseInput {
                button: winit::event::MouseButton::Left,
                state: winit::event::ElementState::Released,
            },
        ]
    }
}

/// Utilities for testing audio/video synchronization
pub mod sync_test {
    use std::time::{Duration, Instant};
    
    pub struct SyncTester {
        start_time: Instant,
        video_frames: Vec<(Duration, u32)>, // (presentation_time, frame_number)
        audio_samples: Vec<(Duration, f32)>, // (presentation_time, sample_value)
    }
    
    impl SyncTester {
        pub fn new() -> Self {
            Self {
                start_time: Instant::now(),
                video_frames: Vec::new(),
                audio_samples: Vec::new(),
            }
        }
        
        pub fn record_video_frame(&mut self, frame_number: u32) {
            let elapsed = self.start_time.elapsed();
            self.video_frames.push((elapsed, frame_number));
        }
        
        pub fn record_audio_sample(&mut self, sample: f32) {
            let elapsed = self.start_time.elapsed();
            self.audio_samples.push((elapsed, sample));
        }
        
        /// Calculate the synchronization offset between audio and video
        pub fn calculate_sync_offset(&self) -> Duration {
            // Simple implementation - in reality this would be more sophisticated
            if self.video_frames.is_empty() || self.audio_samples.is_empty() {
                return Duration::ZERO;
            }
            
            let first_video = self.video_frames.first().unwrap().0;
            let first_audio = self.audio_samples.first().unwrap().0;
            
            first_video.saturating_sub(first_audio)
        }
        
        /// Check if A/V sync is within acceptable bounds (typically 40ms)
        pub fn is_synced(&self, tolerance: Duration) -> bool {
            self.calculate_sync_offset() <= tolerance
        }
    }
}

/// Performance measurement utilities
pub mod perf_test {
    use std::time::{Duration, Instant};
    
    pub struct PerfMeasure {
        name: String,
        start: Instant,
        measurements: Vec<Duration>,
    }
    
    impl PerfMeasure {
        pub fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                start: Instant::now(),
                measurements: Vec::new(),
            }
        }
        
        pub fn start(&mut self) {
            self.start = Instant::now();
        }
        
        pub fn stop(&mut self) {
            self.measurements.push(self.start.elapsed());
        }
        
        pub fn average(&self) -> Duration {
            if self.measurements.is_empty() {
                return Duration::ZERO;
            }
            
            let sum: Duration = self.measurements.iter().sum();
            sum / self.measurements.len() as u32
        }
        
        pub fn min(&self) -> Option<Duration> {
            self.measurements.iter().min().copied()
        }
        
        pub fn max(&self) -> Option<Duration> {
            self.measurements.iter().max().copied()
        }
        
        pub fn report(&self) {
            println!("Performance Report: {}", self.name);
            println!("  Samples: {}", self.measurements.len());
            println!("  Average: {:?}", self.average());
            println!("  Min: {:?}", self.min().unwrap_or(Duration::ZERO));
            println!("  Max: {:?}", self.max().unwrap_or(Duration::ZERO));
        }
    }
}