//! CPAL audio output implementation for CCPlayer
//! 
//! This module implements the AudioOutput trait using the cpal library
//! for cross-platform, low-latency audio playback.

use crate::audio::{
    AudioDevice, AudioDeviceType, AudioEventHandler, AudioFormat, AudioOutput, 
    AudioProcessingOptions, AudioStats, ChannelLayout, SampleFormat,
};
use crate::decoder::AudioSamples;
use crate::utils::error::{CCPlayerError, IntoPlayerError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, StreamConfig, SampleRate};
use crossbeam::channel::{bounded, Receiver, Sender};
use parking_lot::{Mutex, RwLock};
use ringbuf::{HeapRb, Producer, Consumer};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Ring buffer size in samples per channel
const RING_BUFFER_SIZE: usize = 8192;

/// Target buffer fill level (50% of buffer)
const TARGET_FILL_RATIO: f32 = 0.5;

/// Minimum buffer fill before starting playback (25% of buffer)
const MIN_FILL_RATIO: f32 = 0.25;

/// Volume ramp duration for smooth transitions (in samples)
const VOLUME_RAMP_SAMPLES: usize = 512;

/// CPAL audio output implementation
pub struct CpalAudioOutput {
    /// Audio format
    format: Option<AudioFormat>,
    
    /// CPAL host
    host: Host,
    
    /// Current audio device
    device: Option<Device>,
    
    /// Audio stream
    stream: Option<Stream>,
    
    /// Ring buffer producer for sending audio samples
    producer: Option<Arc<Mutex<Producer<f32, Arc<HeapRb<f32>>>>>>,
    
    /// Volume control (0.0 to 1.0)
    volume: Arc<RwLock<VolumeControl>>,
    
    /// Playback state
    state: Arc<RwLock<PlaybackState>>,
    
    /// Audio clock for synchronization
    clock: Arc<AudioClock>,
    
    /// Audio statistics
    stats: Arc<RwLock<AudioStats>>,
    
    /// Event handler
    event_handler: Option<Arc<Mutex<dyn AudioEventHandler>>>,
    
    /// Processing options
    processing_options: Arc<RwLock<AudioProcessingOptions>>,
    
    /// Device monitor thread handle
    device_monitor: Option<thread::JoinHandle<()>>,
    
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
}

/// Volume control with smooth transitions
struct VolumeControl {
    /// Current volume level
    current: f32,
    
    /// Target volume level
    target: f32,
    
    /// Volume ramp samples remaining
    ramp_samples: usize,
}

impl VolumeControl {
    fn new() -> Self {
        Self {
            current: 1.0,
            target: 1.0,
            ramp_samples: 0,
        }
    }
    
    /// Process volume for a sample with smooth ramping
    fn process(&mut self, sample: f32) -> f32 {
        if self.ramp_samples > 0 {
            let step = (self.target - self.current) / self.ramp_samples as f32;
            self.current += step;
            self.ramp_samples -= 1;
            
            if self.ramp_samples == 0 {
                self.current = self.target;
            }
        }
        
        sample * self.current
    }
    
    /// Set target volume with smooth transition
    fn set_target(&mut self, volume: f32) {
        self.target = volume.clamp(0.0, 1.0);
        self.ramp_samples = VOLUME_RAMP_SAMPLES;
    }
}

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    Buffering,
}

/// Audio clock for A/V synchronization
struct AudioClock {
    /// Sample rate
    sample_rate: AtomicU64,
    
    /// Total samples played
    samples_played: AtomicU64,
    
    /// Current PTS (presentation timestamp)
    current_pts: AtomicI64,
    
    /// Start time
    start_time: Mutex<Option<Instant>>,
    
    /// Pause time
    pause_time: Mutex<Option<Instant>>,
    
    /// Total pause duration
    pause_duration: Mutex<Duration>,
}

impl AudioClock {
    fn new() -> Self {
        Self {
            sample_rate: AtomicU64::new(48000),
            samples_played: AtomicU64::new(0),
            current_pts: AtomicI64::new(0),
            start_time: Mutex::new(None),
            pause_time: Mutex::new(None),
            pause_duration: Mutex::new(Duration::ZERO),
        }
    }
    
    /// Update clock with played samples
    fn update_samples(&self, samples: u64) {
        self.samples_played.fetch_add(samples, Ordering::Relaxed);
    }
    
    /// Get current playback position in microseconds
    fn get_position(&self) -> i64 {
        let samples = self.samples_played.load(Ordering::Relaxed);
        let sample_rate = self.sample_rate.load(Ordering::Relaxed);
        
        if sample_rate == 0 {
            return 0;
        }
        
        // Convert samples to microseconds
        (samples as i64 * 1_000_000) / sample_rate as i64
    }
    
    /// Start the clock
    fn start(&self) {
        let mut start_time = self.start_time.lock();
        *start_time = Some(Instant::now());
    }
    
    /// Pause the clock
    fn pause(&self) {
        let mut pause_time = self.pause_time.lock();
        *pause_time = Some(Instant::now());
    }
    
    /// Resume the clock
    fn resume(&self) {
        let mut pause_time = self.pause_time.lock();
        if let Some(pt) = pause_time.take() {
            let mut pause_duration = self.pause_duration.lock();
            *pause_duration += pt.elapsed();
        }
    }
    
    /// Reset the clock
    fn reset(&self) {
        self.samples_played.store(0, Ordering::Relaxed);
        self.current_pts.store(0, Ordering::Relaxed);
        *self.start_time.lock() = None;
        *self.pause_time.lock() = None;
        *self.pause_duration.lock() = Duration::ZERO;
    }
}

impl AudioOutput for CpalAudioOutput {
    fn new() -> Result<Self> where Self: Sized {
        let host = cpal::default_host();
        
        Ok(Self {
            format: None,
            host,
            device: None,
            stream: None,
            producer: None,
            volume: Arc::new(RwLock::new(VolumeControl::new())),
            state: Arc::new(RwLock::new(PlaybackState::Stopped)),
            clock: Arc::new(AudioClock::new()),
            stats: Arc::new(RwLock::new(AudioStats::default())),
            event_handler: None,
            processing_options: Arc::new(RwLock::new(AudioProcessingOptions::default())),
            device_monitor: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }
    
    fn initialize(&mut self, format: AudioFormat) -> Result<()> {
        // Stop any existing stream
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
        
        // Select default output device
        let device = self.host.default_output_device()
            .ok_or_else(|| CCPlayerError::Audio("No default output device found".to_string()))?;
        
        // Create stream config
        let config = StreamConfig {
            channels: format.channels,
            sample_rate: SampleRate(format.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(256), // Low latency buffer
        };
        
        // Create ring buffer
        let ring_buffer = HeapRb::<f32>::new(RING_BUFFER_SIZE * format.channels as usize);
        let (producer, mut consumer) = ring_buffer.split();
        
        // Store producer
        self.producer = Some(Arc::new(Mutex::new(producer)));
        
        // Clone for stream callback
        let volume = Arc::clone(&self.volume);
        let state = Arc::clone(&self.state);
        let clock = Arc::clone(&self.clock);
        let stats = Arc::clone(&self.stats);
        let channels = format.channels as usize;
        
        // Update clock sample rate
        self.clock.sample_rate.store(format.sample_rate as u64, Ordering::Relaxed);
        
        // Create output stream
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let current_state = *state.read();
                
                match current_state {
                    PlaybackState::Playing => {
                        let samples_needed = data.len();
                        let samples_available = consumer.len();
                        
                        if samples_available >= samples_needed {
                            // Read samples from ring buffer
                            for sample in data.iter_mut() {
                                if let Some(s) = consumer.pop() {
                                    // Apply volume
                                    *sample = volume.write().process(s);
                                } else {
                                    *sample = 0.0;
                                }
                            }
                            
                            // Update clock
                            let frames = samples_needed / channels;
                            clock.update_samples(frames as u64);
                            
                            // Update stats
                            let mut stats_guard = stats.write();
                            stats_guard.samples_played += frames as u64;
                        } else {
                            // Buffer underrun
                            data.fill(0.0);
                            
                            let mut stats_guard = stats.write();
                            stats_guard.underruns += 1;
                            
                            // Switch to buffering state
                            *state.write() = PlaybackState::Buffering;
                        }
                    },
                    PlaybackState::Paused | PlaybackState::Stopped | PlaybackState::Buffering => {
                        // Output silence
                        data.fill(0.0);
                    }
                }
            },
            |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None
        ).audio_err("Failed to create audio stream")?;
        
        // Start the stream
        stream.play().audio_err("Failed to start audio stream")?;
        
        // Store device and stream
        self.device = Some(device);
        self.stream = Some(stream);
        self.format = Some(format);
        
        // Update stats
        let mut stats = self.stats.write();
        stats.sample_rate = format.sample_rate;
        stats.bit_depth = match format.sample_format {
            SampleFormat::U8 => 8,
            SampleFormat::I16 => 16,
            SampleFormat::I24 => 24,
            SampleFormat::I32 | SampleFormat::F32 => 32,
            SampleFormat::F64 => 64,
        };
        
        Ok(())
    }
    
    fn play(&mut self, samples: &AudioSamples) -> Result<()> {
        let producer = self.producer.as_ref()
            .ok_or_else(|| CCPlayerError::Audio("Audio not initialized".to_string()))?;
        
        let format = self.format.as_ref()
            .ok_or_else(|| CCPlayerError::Audio("Audio format not set".to_string()))?;
        
        // Validate sample format
        if samples.channels != format.channels as usize {
            return Err(CCPlayerError::Audio(format!(
                "Channel count mismatch: expected {}, got {}",
                format.channels, samples.channels
            )));
        }
        
        // Resample if necessary
        let resampled = if samples.sample_rate != format.sample_rate {
            resample_audio(samples, format.sample_rate)?
        } else {
            samples.data.clone()
        };
        
        // Write samples to ring buffer
        let mut producer_guard = producer.lock();
        let mut written = 0;
        
        for sample in resampled.iter() {
            if producer_guard.push(*sample).is_ok() {
                written += 1;
            } else {
                // Buffer full, drop remaining samples
                break;
            }
        }
        
        // Update PTS
        self.clock.current_pts.store(samples.pts, Ordering::Relaxed);
        
        // Check buffer fill level
        let fill_ratio = producer_guard.len() as f32 / producer_guard.capacity() as f32;
        
        // Start playback if we have enough data
        let mut state = self.state.write();
        if *state == PlaybackState::Buffering && fill_ratio >= MIN_FILL_RATIO {
            *state = PlaybackState::Playing;
            self.clock.start();
        }
        
        Ok(())
    }
    
    fn pause(&mut self) -> Result<()> {
        let mut state = self.state.write();
        if *state == PlaybackState::Playing {
            *state = PlaybackState::Paused;
            self.clock.pause();
        }
        Ok(())
    }
    
    fn resume(&mut self) -> Result<()> {
        let mut state = self.state.write();
        if *state == PlaybackState::Paused {
            *state = PlaybackState::Playing;
            self.clock.resume();
        }
        Ok(())
    }
    
    fn stop(&mut self) -> Result<()> {
        // Stop playback
        *self.state.write() = PlaybackState::Stopped;
        
        // Clear ring buffer
        if let Some(producer) = &self.producer {
            let mut producer_guard = producer.lock();
            producer_guard.clear();
        }
        
        // Reset clock
        self.clock.reset();
        
        // Reset stats
        self.stats.write().underruns = 0;
        
        Ok(())
    }
    
    fn set_volume(&mut self, volume: f32) -> Result<()> {
        self.volume.write().set_target(volume);
        Ok(())
    }
    
    fn get_volume(&self) -> f32 {
        self.volume.read().target
    }
    
    fn get_position(&self) -> i64 {
        self.clock.get_position()
    }
    
    fn get_latency(&self) -> i64 {
        // Estimate latency based on buffer size and fill level
        if let Some(producer) = &self.producer {
            let format = self.format.as_ref().unwrap_or(&AudioFormat::default());
            let producer_guard = producer.lock();
            let buffered_samples = producer_guard.len() / format.channels as usize;
            let buffered_us = (buffered_samples as i64 * 1_000_000) / format.sample_rate as i64;
            
            // Add estimated device latency (conservative estimate)
            buffered_us + 10_000 // 10ms device latency
        } else {
            0
        }
    }
    
    fn is_playing(&self) -> bool {
        *self.state.read() == PlaybackState::Playing
    }
    
    fn get_buffer_fill(&self) -> f32 {
        if let Some(producer) = &self.producer {
            let producer_guard = producer.lock();
            producer_guard.len() as f32 / producer_guard.capacity() as f32
        } else {
            0.0
        }
    }
    
    fn set_device(&mut self, device: AudioDevice) -> Result<()> {
        // Find the device
        let devices = self.host.output_devices()
            .audio_err("Failed to enumerate output devices")?;
        
        for dev in devices {
            if let Ok(name) = dev.name() {
                if name == device.name || device.id == name {
                    // Re-initialize with new device
                    if let Some(format) = self.format {
                        self.device = Some(dev);
                        return self.initialize(format);
                    }
                }
            }
        }
        
        Err(CCPlayerError::Audio(format!("Device not found: {}", device.name)))
    }
    
    fn get_devices() -> Result<Vec<AudioDevice>> {
        let host = cpal::default_host();
        let mut devices = Vec::new();
        
        // Get default device name
        let default_name = host.default_output_device()
            .and_then(|d| d.name().ok());
        
        // Enumerate all output devices
        for device in host.output_devices().audio_err("Failed to enumerate output devices")? {
            if let Ok(name) = device.name() {
                let is_default = Some(&name) == default_name.as_ref();
                
                // Get supported configurations
                let mut sample_rates = Vec::new();
                let mut max_channels = 0u16;
                
                if let Ok(configs) = device.supported_output_configs() {
                    for config in configs {
                        // Add sample rates
                        sample_rates.push(config.min_sample_rate().0);
                        if config.max_sample_rate().0 != config.min_sample_rate().0 {
                            sample_rates.push(config.max_sample_rate().0);
                        }
                        
                        // Track max channels
                        max_channels = max_channels.max(config.channels());
                    }
                }
                
                // Remove duplicates and sort
                sample_rates.sort_unstable();
                sample_rates.dedup();
                
                // Determine device type
                let device_type = guess_device_type(&name);
                
                devices.push(AudioDevice {
                    name: name.clone(),
                    id: name,
                    is_default,
                    sample_rates,
                    max_channels,
                    device_type,
                });
            }
        }
        
        Ok(devices)
    }
}

impl Drop for CpalAudioOutput {
    fn drop(&mut self) {
        // Signal shutdown
        self.shutdown.store(true, Ordering::Relaxed);
        
        // Stop device monitor thread
        if let Some(handle) = self.device_monitor.take() {
            let _ = handle.join();
        }
        
        // Stop stream
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
    }
}

/// Resample audio to target sample rate
fn resample_audio(samples: &AudioSamples, target_rate: u32) -> Result<Vec<f32>> {
    let ratio = target_rate as f64 / samples.sample_rate as f64;
    let output_len = (samples.data.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(output_len);
    
    // Simple linear interpolation resampling
    // For production, consider using a proper resampling library
    let channels = samples.channels;
    let input_frames = samples.sample_count;
    let output_frames = (input_frames as f64 * ratio) as usize;
    
    for frame in 0..output_frames {
        let source_frame = (frame as f64 / ratio) as usize;
        let fraction = (frame as f64 / ratio) - source_frame as f64;
        
        for ch in 0..channels {
            let idx = source_frame * channels + ch;
            let next_idx = ((source_frame + 1) * channels + ch).min(samples.data.len() - 1);
            
            // Linear interpolation
            let sample = samples.data[idx] * (1.0 - fraction as f32) +
                        samples.data[next_idx] * fraction as f32;
            output.push(sample);
        }
    }
    
    Ok(output)
}

/// Guess device type from name
fn guess_device_type(name: &str) -> AudioDeviceType {
    let lower = name.to_lowercase();
    
    if lower.contains("speaker") || lower.contains("realtek") {
        AudioDeviceType::Speakers
    } else if lower.contains("headphone") || lower.contains("headset") {
        AudioDeviceType::Headphones
    } else if lower.contains("hdmi") {
        AudioDeviceType::Hdmi
    } else if lower.contains("spdif") || lower.contains("digital") {
        AudioDeviceType::Digital
    } else if lower.contains("bluetooth") || lower.contains("bt") {
        AudioDeviceType::Bluetooth
    } else if lower.contains("usb") {
        AudioDeviceType::Usb
    } else if lower.contains("virtual") || lower.contains("cable") {
        AudioDeviceType::Virtual
    } else {
        AudioDeviceType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_volume_control() {
        let mut vol = VolumeControl::new();
        assert_eq!(vol.current, 1.0);
        
        // Test immediate volume
        let sample = vol.process(0.5);
        assert_eq!(sample, 0.5);
        
        // Test ramping
        vol.set_target(0.5);
        assert_eq!(vol.ramp_samples, VOLUME_RAMP_SAMPLES);
        
        // Process one sample
        let _ = vol.process(1.0);
        assert!(vol.current < 1.0);
        assert!(vol.current > 0.5);
    }
    
    #[test]
    fn test_audio_clock() {
        let clock = AudioClock::new();
        
        // Test initial state
        assert_eq!(clock.get_position(), 0);
        
        // Test sample updates
        clock.sample_rate.store(48000, Ordering::Relaxed);
        clock.update_samples(48000); // 1 second of samples
        assert_eq!(clock.get_position(), 1_000_000); // 1 second in microseconds
    }
    
    #[test]
    fn test_device_type_detection() {
        assert_eq!(guess_device_type("Realtek HD Audio"), AudioDeviceType::Speakers);
        assert_eq!(guess_device_type("USB Headphones"), AudioDeviceType::Headphones);
        assert_eq!(guess_device_type("HDMI Output"), AudioDeviceType::Hdmi);
        assert_eq!(guess_device_type("Bluetooth Speaker"), AudioDeviceType::Bluetooth);
        assert_eq!(guess_device_type("Unknown Device"), AudioDeviceType::Unknown);
    }
}