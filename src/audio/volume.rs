//! Volume control implementation for CCPlayer
//! 
//! This module provides volume control with smooth transitions,
//! normalization, and dynamic range compression.

use crate::utils::error::{CCPlayerError, Result};
use parking_lot::RwLock;
use std::sync::Arc;

/// Volume controller with various processing options
pub struct VolumeController {
    /// Master volume (0.0 to 1.0)
    master_volume: Arc<RwLock<f32>>,
    
    /// Channel volumes for multi-channel control
    channel_volumes: Arc<RwLock<Vec<f32>>>,
    
    /// Mute state
    muted: Arc<RwLock<bool>>,
    
    /// Volume before muting (for unmute)
    pre_mute_volume: Arc<RwLock<f32>>,
    
    /// Normalization processor
    normalizer: Arc<RwLock<Normalizer>>,
    
    /// Dynamic range compressor
    compressor: Arc<RwLock<Compressor>>,
    
    /// Volume ramping for smooth transitions
    ramp: Arc<RwLock<VolumeRamp>>,
}

/// Audio normalizer for consistent loudness
pub struct Normalizer {
    /// Enable normalization
    enabled: bool,
    
    /// Target LUFS level
    target_lufs: f32,
    
    /// Current loudness measurement
    current_lufs: f32,
    
    /// Gain to apply
    gain: f32,
    
    /// Measurement window in samples
    window_size: usize,
    
    /// Sample buffer for measurement
    buffer: Vec<f32>,
    
    /// Buffer position
    buffer_pos: usize,
}

/// Dynamic range compressor
pub struct Compressor {
    /// Enable compression
    enabled: bool,
    
    /// Threshold in dB
    threshold_db: f32,
    
    /// Compression ratio (e.g., 4:1)
    ratio: f32,
    
    /// Attack time in milliseconds
    attack_ms: f32,
    
    /// Release time in milliseconds
    release_ms: f32,
    
    /// Makeup gain in dB
    makeup_gain_db: f32,
    
    /// Current envelope level
    envelope: f32,
    
    /// Sample rate for time calculations
    sample_rate: u32,
}

/// Volume ramping for smooth transitions
pub struct VolumeRamp {
    /// Current volume
    current: f32,
    
    /// Target volume
    target: f32,
    
    /// Ramp duration in samples
    duration_samples: usize,
    
    /// Samples processed
    samples_processed: usize,
    
    /// Ramp type
    ramp_type: RampType,
}

/// Volume ramp type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RampType {
    /// Linear ramp
    Linear,
    
    /// Exponential ramp (more natural)
    Exponential,
    
    /// S-curve ramp (smoothest)
    SCurve,
}

impl VolumeController {
    /// Create a new volume controller
    pub fn new(channels: usize) -> Self {
        let channel_volumes = vec![1.0; channels];
        
        Self {
            master_volume: Arc::new(RwLock::new(1.0)),
            channel_volumes: Arc::new(RwLock::new(channel_volumes)),
            muted: Arc::new(RwLock::new(false)),
            pre_mute_volume: Arc::new(RwLock::new(1.0)),
            normalizer: Arc::new(RwLock::new(Normalizer::new())),
            compressor: Arc::new(RwLock::new(Compressor::new(48000))),
            ramp: Arc::new(RwLock::new(VolumeRamp::new())),
        }
    }
    
    /// Process audio samples with volume control
    pub fn process(&self, samples: &mut [f32], channels: usize) {
        let is_muted = *self.muted.read();
        if is_muted {
            samples.fill(0.0);
            return;
        }
        
        // Apply normalization if enabled
        if self.normalizer.read().enabled {
            self.apply_normalization(samples);
        }
        
        // Apply compression if enabled
        if self.compressor.read().enabled {
            self.apply_compression(samples);
        }
        
        // Apply volume with ramping
        self.apply_volume(samples, channels);
    }
    
    /// Apply normalization
    fn apply_normalization(&self, samples: &mut [f32]) {
        let mut normalizer = self.normalizer.write();
        
        // Update loudness measurement
        normalizer.update_measurement(samples);
        
        // Apply gain
        let gain = normalizer.gain;
        for sample in samples.iter_mut() {
            *sample *= gain;
        }
    }
    
    /// Apply dynamic range compression
    fn apply_compression(&self, samples: &mut [f32]) {
        let mut compressor = self.compressor.write();
        
        for sample in samples.iter_mut() {
            *sample = compressor.process_sample(*sample);
        }
    }
    
    /// Apply volume with channel control and ramping
    fn apply_volume(&self, samples: &mut [f32], channels: usize) {
        let mut ramp = self.ramp.write();
        let master = *self.master_volume.read();
        let channel_vols = self.channel_volumes.read();
        
        for (i, sample) in samples.iter_mut().enumerate() {
            let channel = i % channels;
            let channel_vol = channel_vols.get(channel).copied().unwrap_or(1.0);
            
            // Get ramped volume
            let ramped_volume = ramp.next_value();
            
            // Apply all volume stages
            *sample *= ramped_volume * master * channel_vol;
        }
    }
    
    /// Set master volume (0.0 to 1.0)
    pub fn set_volume(&self, volume: f32) -> Result<()> {
        let clamped = volume.clamp(0.0, 1.0);
        *self.master_volume.write() = clamped;
        
        // Update ramp target
        self.ramp.write().set_target(clamped);
        
        // Update pre-mute volume if not muted
        if !*self.muted.read() {
            *self.pre_mute_volume.write() = clamped;
        }
        
        Ok(())
    }
    
    /// Get current volume
    pub fn get_volume(&self) -> f32 {
        *self.master_volume.read()
    }
    
    /// Set channel volume
    pub fn set_channel_volume(&self, channel: usize, volume: f32) -> Result<()> {
        let mut channel_vols = self.channel_volumes.write();
        if channel >= channel_vols.len() {
            return Err(CCPlayerError::Audio(format!(
                "Invalid channel index: {}",
                channel
            )));
        }
        
        channel_vols[channel] = volume.clamp(0.0, 1.0);
        Ok(())
    }
    
    /// Get channel volume
    pub fn get_channel_volume(&self, channel: usize) -> Result<f32> {
        let channel_vols = self.channel_volumes.read();
        channel_vols.get(channel).copied()
            .ok_or_else(|| CCPlayerError::Audio(format!(
                "Invalid channel index: {}",
                channel
            )))
    }
    
    /// Mute audio
    pub fn mute(&self) {
        *self.pre_mute_volume.write() = *self.master_volume.read();
        *self.muted.write() = true;
    }
    
    /// Unmute audio
    pub fn unmute(&self) {
        *self.muted.write() = false;
        let pre_mute = *self.pre_mute_volume.read();
        self.set_volume(pre_mute).ok();
    }
    
    /// Toggle mute state
    pub fn toggle_mute(&self) {
        if *self.muted.read() {
            self.unmute();
        } else {
            self.mute();
        }
    }
    
    /// Check if muted
    pub fn is_muted(&self) -> bool {
        *self.muted.read()
    }
    
    /// Set normalization enabled
    pub fn set_normalization_enabled(&self, enabled: bool) {
        self.normalizer.write().enabled = enabled;
    }
    
    /// Set normalization target
    pub fn set_normalization_target(&self, target_lufs: f32) {
        self.normalizer.write().target_lufs = target_lufs;
    }
    
    /// Set compression enabled
    pub fn set_compression_enabled(&self, enabled: bool) {
        self.compressor.write().enabled = enabled;
    }
    
    /// Set compression parameters
    pub fn set_compression_params(
        &self,
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        makeup_gain_db: f32,
    ) {
        let mut comp = self.compressor.write();
        comp.threshold_db = threshold_db;
        comp.ratio = ratio.max(1.0);
        comp.attack_ms = attack_ms.max(0.1);
        comp.release_ms = release_ms.max(1.0);
        comp.makeup_gain_db = makeup_gain_db;
    }
    
    /// Set volume ramp duration in milliseconds
    pub fn set_ramp_duration_ms(&self, duration_ms: f32, sample_rate: u32) {
        let samples = ((duration_ms / 1000.0) * sample_rate as f32) as usize;
        self.ramp.write().duration_samples = samples;
    }
    
    /// Set volume ramp type
    pub fn set_ramp_type(&self, ramp_type: RampType) {
        self.ramp.write().ramp_type = ramp_type;
    }
}

impl Normalizer {
    fn new() -> Self {
        Self {
            enabled: false,
            target_lufs: -14.0, // Standard for streaming
            current_lufs: -14.0,
            gain: 1.0,
            window_size: 48000 * 3, // 3 seconds at 48kHz
            buffer: Vec::with_capacity(48000 * 3),
            buffer_pos: 0,
        }
    }
    
    /// Update loudness measurement
    fn update_measurement(&mut self, samples: &[f32]) {
        // Simple RMS-based measurement (real LUFS would be more complex)
        for &sample in samples {
            if self.buffer.len() < self.window_size {
                self.buffer.push(sample);
            } else {
                self.buffer[self.buffer_pos] = sample;
                self.buffer_pos = (self.buffer_pos + 1) % self.window_size;
            }
        }
        
        // Calculate RMS
        if !self.buffer.is_empty() {
            let sum_squares: f32 = self.buffer.iter().map(|s| s * s).sum();
            let rms = (sum_squares / self.buffer.len() as f32).sqrt();
            
            // Convert to dB (simplified)
            let db = 20.0 * rms.max(0.00001).log10();
            self.current_lufs = db;
            
            // Calculate gain needed
            let target_linear = 10.0_f32.powf(self.target_lufs / 20.0);
            let current_linear = 10.0_f32.powf(db / 20.0);
            self.gain = (target_linear / current_linear).clamp(0.1, 10.0);
        }
    }
}

impl Compressor {
    fn new(sample_rate: u32) -> Self {
        Self {
            enabled: false,
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            makeup_gain_db: 0.0,
            envelope: 0.0,
            sample_rate,
        }
    }
    
    /// Process a single sample
    fn process_sample(&mut self, input: f32) -> f32 {
        let input_abs = input.abs();
        let input_db = 20.0 * input_abs.max(0.00001).log10();
        
        // Envelope follower
        let attack_coeff = (-1.0 / (self.attack_ms * 0.001 * self.sample_rate as f32)).exp();
        let release_coeff = (-1.0 / (self.release_ms * 0.001 * self.sample_rate as f32)).exp();
        
        let target_envelope = if input_db > self.threshold_db {
            input_db
        } else {
            self.threshold_db
        };
        
        if target_envelope > self.envelope {
            // Attack
            self.envelope = target_envelope + (self.envelope - target_envelope) * attack_coeff;
        } else {
            // Release
            self.envelope = target_envelope + (self.envelope - target_envelope) * release_coeff;
        }
        
        // Calculate gain reduction
        let mut gain_db = 0.0;
        if self.envelope > self.threshold_db {
            let excess_db = self.envelope - self.threshold_db;
            gain_db = -excess_db * (1.0 - 1.0 / self.ratio);
        }
        
        // Apply makeup gain
        gain_db += self.makeup_gain_db;
        
        // Convert to linear and apply
        let gain = 10.0_f32.powf(gain_db / 20.0);
        input * gain
    }
}

impl VolumeRamp {
    fn new() -> Self {
        Self {
            current: 1.0,
            target: 1.0,
            duration_samples: 2048,
            samples_processed: 0,
            ramp_type: RampType::Exponential,
        }
    }
    
    /// Set target volume
    fn set_target(&mut self, target: f32) {
        if (target - self.current).abs() > 0.001 {
            self.target = target;
            self.samples_processed = 0;
        }
    }
    
    /// Get next ramped value
    fn next_value(&mut self) -> f32 {
        if self.samples_processed >= self.duration_samples {
            self.current = self.target;
            return self.current;
        }
        
        let progress = self.samples_processed as f32 / self.duration_samples as f32;
        self.samples_processed += 1;
        
        let factor = match self.ramp_type {
            RampType::Linear => progress,
            RampType::Exponential => {
                // Exponential curve for more natural volume changes
                if self.target > self.current {
                    1.0 - (1.0 - progress).powf(2.0)
                } else {
                    progress.powf(2.0)
                }
            }
            RampType::SCurve => {
                // S-curve for smoothest transitions
                let t = progress;
                t * t * (3.0 - 2.0 * t)
            }
        };
        
        self.current + (self.target - self.current) * factor
    }
}

/// Convert between dB and linear scale
pub mod db {
    /// Convert decibels to linear scale
    pub fn to_linear(db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }
    
    /// Convert linear scale to decibels
    pub fn from_linear(linear: f32) -> f32 {
        20.0 * linear.max(0.00001).log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_volume_controller() {
        let controller = VolumeController::new(2);
        
        // Test initial state
        assert_eq!(controller.get_volume(), 1.0);
        assert!(!controller.is_muted());
        
        // Test volume setting
        controller.set_volume(0.5).unwrap();
        assert_eq!(controller.get_volume(), 0.5);
        
        // Test muting
        controller.mute();
        assert!(controller.is_muted());
        
        controller.unmute();
        assert!(!controller.is_muted());
        assert_eq!(controller.get_volume(), 0.5);
    }
    
    #[test]
    fn test_channel_volume() {
        let controller = VolumeController::new(2);
        
        // Test channel volume
        controller.set_channel_volume(0, 0.7).unwrap();
        assert_eq!(controller.get_channel_volume(0).unwrap(), 0.7);
        
        // Test invalid channel
        assert!(controller.set_channel_volume(5, 0.5).is_err());
    }
    
    #[test]
    fn test_volume_ramp() {
        let mut ramp = VolumeRamp::new();
        ramp.duration_samples = 10;
        ramp.set_target(0.0);
        
        // Test ramping down
        let mut values = Vec::new();
        for _ in 0..10 {
            values.push(ramp.next_value());
        }
        
        // Should decrease from 1.0 to 0.0
        assert!(values[0] > values[9]);
        assert!(values[9] < 0.1);
    }
    
    #[test]
    fn test_db_conversion() {
        use db::*;
        
        assert!((to_linear(0.0) - 1.0).abs() < 0.001);
        assert!((to_linear(-6.0) - 0.5).abs() < 0.01);
        assert!((from_linear(1.0) - 0.0).abs() < 0.001);
        assert!((from_linear(0.5) - (-6.0)).abs() < 0.1);
    }
}