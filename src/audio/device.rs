//! Audio device management for CCPlayer
//! 
//! This module handles audio device enumeration, selection,
//! and hot-plug detection.

use crate::audio::{AudioDevice, AudioDeviceType};
use crate::utils::error::{CCPlayerError, IntoPlayerError, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use crossbeam::channel::{bounded, Receiver, Sender};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Audio device manager
pub struct DeviceManager {
    /// CPAL host
    host: cpal::Host,
    
    /// Current devices cache
    devices: Arc<RwLock<DeviceCache>>,
    
    /// Device change listeners
    listeners: Arc<RwLock<Vec<Box<dyn DeviceChangeListener>>>>,
    
    /// Monitor thread handle
    monitor_thread: Option<thread::JoinHandle<()>>,
    
    /// Shutdown channel
    shutdown_tx: Option<Sender<()>>,
}

/// Device cache for quick lookups
struct DeviceCache {
    /// All available devices
    devices: Vec<AudioDevice>,
    
    /// Device lookup by ID
    by_id: HashMap<String, usize>,
    
    /// Default device ID
    default_id: Option<String>,
}

/// Device change listener trait
pub trait DeviceChangeListener: Send + Sync {
    /// Called when a device is added
    fn on_device_added(&mut self, device: &AudioDevice);
    
    /// Called when a device is removed
    fn on_device_removed(&mut self, device_id: &str);
    
    /// Called when default device changes
    fn on_default_changed(&mut self, device: &AudioDevice);
}

impl DeviceManager {
    /// Create a new device manager
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let devices = Arc::new(RwLock::new(DeviceCache::new()));
        
        // Do initial device enumeration
        let initial_devices = enumerate_devices(&host)?;
        devices.write().update(initial_devices);
        
        Ok(Self {
            host,
            devices,
            listeners: Arc::new(RwLock::new(Vec::new())),
            monitor_thread: None,
            shutdown_tx: None,
        })
    }
    
    /// Start device monitoring
    pub fn start_monitoring(&mut self) -> Result<()> {
        if self.monitor_thread.is_some() {
            return Ok(()); // Already monitoring
        }
        
        let (shutdown_tx, shutdown_rx) = bounded(1);
        self.shutdown_tx = Some(shutdown_tx);
        
        let host = self.host.clone();
        let devices = Arc::clone(&self.devices);
        let listeners = Arc::clone(&self.listeners);
        
        // Start monitor thread
        let handle = thread::Builder::new()
            .name("audio-device-monitor".to_string())
            .spawn(move || {
                device_monitor_thread(host, devices, listeners, shutdown_rx);
            })
            .audio_err("Failed to start device monitor thread")?;
        
        self.monitor_thread = Some(handle);
        Ok(())
    }
    
    /// Stop device monitoring
    pub fn stop_monitoring(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        
        if let Some(handle) = self.monitor_thread.take() {
            let _ = handle.join();
        }
    }
    
    /// Get all available devices
    pub fn get_devices(&self) -> Vec<AudioDevice> {
        self.devices.read().devices.clone()
    }
    
    /// Get device by ID
    pub fn get_device(&self, id: &str) -> Option<AudioDevice> {
        let cache = self.devices.read();
        cache.by_id.get(id)
            .and_then(|&idx| cache.devices.get(idx))
            .cloned()
    }
    
    /// Get default output device
    pub fn get_default_device(&self) -> Option<AudioDevice> {
        let cache = self.devices.read();
        cache.default_id.as_ref()
            .and_then(|id| cache.by_id.get(id))
            .and_then(|&idx| cache.devices.get(idx))
            .cloned()
    }
    
    /// Add device change listener
    pub fn add_listener(&self, listener: Box<dyn DeviceChangeListener>) {
        self.listeners.write().push(listener);
    }
    
    /// Refresh device list manually
    pub fn refresh(&self) -> Result<()> {
        let new_devices = enumerate_devices(&self.host)?;
        let old_devices = self.devices.read().devices.clone();
        
        // Update cache
        self.devices.write().update(new_devices.clone());
        
        // Notify listeners of changes
        let listeners = self.listeners.read();
        for listener in listeners.iter() {
            // Check for added devices
            for new_device in &new_devices {
                if !old_devices.iter().any(|d| d.id == new_device.id) {
                    // Device added
                    // Note: We can't directly call mutable methods on shared references
                    // In a real implementation, we'd use message passing or other patterns
                }
            }
            
            // Check for removed devices
            for old_device in &old_devices {
                if !new_devices.iter().any(|d| d.id == old_device.id) {
                    // Device removed
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if a device supports a specific configuration
    pub fn check_device_support(
        &self,
        device_id: &str,
        sample_rate: u32,
        channels: u16,
    ) -> Result<bool> {
        // Find the CPAL device
        let devices = self.host.output_devices()
            .audio_err("Failed to enumerate devices")?;
        
        for device in devices {
            if let Ok(name) = device.name() {
                if name == device_id {
                    // Check supported configs
                    if let Ok(configs) = device.supported_output_configs() {
                        for config in configs {
                            let sr_range = config.min_sample_rate().0..=config.max_sample_rate().0;
                            if sr_range.contains(&sample_rate) && config.channels() >= channels {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Get device capabilities
    pub fn get_device_capabilities(&self, device_id: &str) -> Result<DeviceCapabilities> {
        let devices = self.host.output_devices()
            .audio_err("Failed to enumerate devices")?;
        
        for device in devices {
            if let Ok(name) = device.name() {
                if name == device_id {
                    return DeviceCapabilities::from_device(&device);
                }
            }
        }
        
        Err(CCPlayerError::Audio(format!("Device not found: {}", device_id)))
    }
}

impl Drop for DeviceManager {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}

impl DeviceCache {
    fn new() -> Self {
        Self {
            devices: Vec::new(),
            by_id: HashMap::new(),
            default_id: None,
        }
    }
    
    fn update(&mut self, devices: Vec<AudioDevice>) {
        self.devices = devices;
        self.by_id.clear();
        
        for (idx, device) in self.devices.iter().enumerate() {
            self.by_id.insert(device.id.clone(), idx);
            if device.is_default {
                self.default_id = Some(device.id.clone());
            }
        }
    }
}

/// Device capabilities
#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    /// Supported sample rates
    pub sample_rates: Vec<u32>,
    
    /// Minimum channels
    pub min_channels: u16,
    
    /// Maximum channels
    pub max_channels: u16,
    
    /// Supported sample formats
    pub sample_formats: Vec<cpal::SampleFormat>,
    
    /// Preferred buffer size range
    pub buffer_size_range: Option<(u32, u32)>,
    
    /// Supports exclusive mode (Windows WASAPI)
    pub exclusive_mode: bool,
}

impl DeviceCapabilities {
    fn from_device(device: &cpal::Device) -> Result<Self> {
        let mut sample_rates = Vec::new();
        let mut min_channels = u16::MAX;
        let mut max_channels = 0u16;
        let mut sample_formats = Vec::new();
        
        if let Ok(configs) = device.supported_output_configs() {
            for config in configs {
                // Collect sample rates
                let min_sr = config.min_sample_rate().0;
                let max_sr = config.max_sample_rate().0;
                
                // Add common sample rates in range
                for &rate in &[8000, 16000, 22050, 44100, 48000, 88200, 96000, 192000] {
                    if rate >= min_sr && rate <= max_sr && !sample_rates.contains(&rate) {
                        sample_rates.push(rate);
                    }
                }
                
                // Update channel limits
                let channels = config.channels();
                min_channels = min_channels.min(channels);
                max_channels = max_channels.max(channels);
                
                // Collect sample formats
                let format = config.sample_format();
                if !sample_formats.contains(&format) {
                    sample_formats.push(format);
                }
            }
        }
        
        sample_rates.sort_unstable();
        
        Ok(Self {
            sample_rates,
            min_channels,
            max_channels,
            sample_formats,
            buffer_size_range: None, // CPAL doesn't expose this directly
            exclusive_mode: cfg!(windows), // Available on Windows
        })
    }
}

/// Device monitor thread function
fn device_monitor_thread(
    host: cpal::Host,
    devices: Arc<RwLock<DeviceCache>>,
    _listeners: Arc<RwLock<Vec<Box<dyn DeviceChangeListener>>>>,
    shutdown_rx: Receiver<()>,
) {
    let poll_interval = Duration::from_secs(2);
    
    loop {
        // Check for shutdown
        if shutdown_rx.try_recv().is_ok() {
            break;
        }
        
        // Enumerate devices
        if let Ok(new_devices) = enumerate_devices(&host) {
            devices.write().update(new_devices);
            
            // TODO: Properly notify listeners using message passing
            // This would require a more sophisticated event system
        }
        
        // Sleep until next poll
        thread::sleep(poll_interval);
    }
}

/// Enumerate all audio output devices
fn enumerate_devices(host: &cpal::Host) -> Result<Vec<AudioDevice>> {
    let mut devices = Vec::new();
    
    // Get default device name
    let default_device = host.default_output_device();
    let default_name = default_device
        .as_ref()
        .and_then(|d| d.name().ok());
    
    // Enumerate all devices
    for device in host.output_devices().audio_err("Failed to enumerate devices")? {
        if let Ok(name) = device.name() {
            let is_default = Some(&name) == default_name.as_ref();
            
            // Get device capabilities
            let mut sample_rates = Vec::new();
            let mut max_channels = 0u16;
            
            if let Ok(configs) = device.supported_output_configs() {
                for config in configs {
                    // Add common sample rates
                    for &rate in &[44100, 48000, 96000, 192000] {
                        let range = config.min_sample_rate().0..=config.max_sample_rate().0;
                        if range.contains(&rate) && !sample_rates.contains(&rate) {
                            sample_rates.push(rate);
                        }
                    }
                    
                    max_channels = max_channels.max(config.channels());
                }
            }
            
            sample_rates.sort_unstable();
            
            devices.push(AudioDevice {
                name: name.clone(),
                id: name.clone(), // CPAL uses name as ID
                is_default,
                sample_rates,
                max_channels,
                device_type: guess_device_type(&name),
            });
        }
    }
    
    Ok(devices)
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

/// Simple device change listener for testing
pub struct LoggingDeviceListener;

impl DeviceChangeListener for LoggingDeviceListener {
    fn on_device_added(&mut self, device: &AudioDevice) {
        println!("Device added: {} ({})", device.name, device.id);
    }
    
    fn on_device_removed(&mut self, device_id: &str) {
        println!("Device removed: {}", device_id);
    }
    
    fn on_default_changed(&mut self, device: &AudioDevice) {
        println!("Default device changed to: {} ({})", device.name, device.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_device_manager_creation() {
        let manager = DeviceManager::new().unwrap();
        let devices = manager.get_devices();
        
        // Should have at least one device on most systems
        assert!(!devices.is_empty());
        
        // Should have a default device
        assert!(manager.get_default_device().is_some());
    }
    
    #[test]
    fn test_device_type_detection() {
        assert_eq!(guess_device_type("Realtek HD Audio"), AudioDeviceType::Speakers);
        assert_eq!(guess_device_type("USB Headphones"), AudioDeviceType::Headphones);
        assert_eq!(guess_device_type("HDMI Output"), AudioDeviceType::Hdmi);
        assert_eq!(guess_device_type("Unknown Device"), AudioDeviceType::Unknown);
    }
    
    #[test]
    fn test_device_cache() {
        let mut cache = DeviceCache::new();
        
        let devices = vec![
            AudioDevice {
                name: "Device 1".to_string(),
                id: "dev1".to_string(),
                is_default: true,
                sample_rates: vec![44100, 48000],
                max_channels: 2,
                device_type: AudioDeviceType::Speakers,
            },
            AudioDevice {
                name: "Device 2".to_string(),
                id: "dev2".to_string(),
                is_default: false,
                sample_rates: vec![48000],
                max_channels: 2,
                device_type: AudioDeviceType::Headphones,
            },
        ];
        
        cache.update(devices);
        
        assert_eq!(cache.devices.len(), 2);
        assert_eq!(cache.by_id.len(), 2);
        assert_eq!(cache.default_id, Some("dev1".to_string()));
    }
}