//! WGPU-based renderer implementation for CCPlayer
//! 
//! This module provides the main renderer implementation using wgpu for
//! high-performance GPU-accelerated video rendering.

use crate::renderer::{
    Color, FrameData, Overlay, OverlayPosition, RenderStats, Renderer, VideoFrame,
};
use crate::utils::error::{CCPlayerError, Result};
use crate::window::Window;
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;

use super::pipeline::RenderPipeline;
use super::texture::TextureManager;

/// WGPU renderer implementation
pub struct WgpuRenderer {
    /// WGPU device
    device: wgpu::Device,
    
    /// WGPU queue for submitting commands
    queue: wgpu::Queue,
    
    /// Surface configuration
    surface_config: wgpu::SurfaceConfiguration,
    
    /// Render surface
    surface: wgpu::Surface<'static>,
    
    /// Render pipeline
    pipeline: RenderPipeline,
    
    /// Texture manager for video frames
    texture_manager: TextureManager,
    
    /// Current window size
    window_size: (u32, u32),
    
    /// Video aspect ratio
    aspect_ratio: f32,
    
    /// Render statistics
    stats: RenderStats,
    
    /// Frame timing for FPS calculation
    frame_times: Vec<Instant>,
    
    /// Last frame time
    last_frame_time: Instant,
}

impl Renderer for WgpuRenderer {
    fn new(window: Arc<dyn Window>) -> Result<Self> 
    where 
        Self: Sized 
    {
        // Initialize wgpu
        let (device, queue, surface, surface_config) = pollster::block_on(
            Self::init_wgpu(window.as_ref())
        )?;
        
        let window_size = (surface_config.width, surface_config.height);
        
        // Create render pipeline
        let pipeline = RenderPipeline::new(&device, surface_config.format)?;
        
        // Create texture manager
        let texture_manager = TextureManager::new(&device)?;
        
        Ok(Self {
            device,
            queue,
            surface_config,
            surface,
            pipeline,
            texture_manager,
            window_size,
            aspect_ratio: 16.0 / 9.0, // Default aspect ratio
            stats: RenderStats::default(),
            frame_times: Vec::with_capacity(120), // Track up to 120 frames for FPS
            last_frame_time: Instant::now(),
        })
    }
    
    fn render_frame(&mut self, frame: VideoFrame) -> Result<()> {
        // Update video texture with new frame data
        self.texture_manager.update_video_texture(
            &self.device,
            &self.queue,
            &frame,
        )?;
        
        // Update aspect ratio if needed
        let frame_aspect = (frame.width as f32 * frame.par) / frame.height as f32;
        if (self.aspect_ratio - frame_aspect).abs() > 0.001 {
            self.aspect_ratio = frame_aspect;
        }
        
        Ok(())
    }
    
    fn render_overlay(&mut self, overlay: Overlay) -> Result<()> {
        // TODO: Implement overlay rendering
        // For now, we'll just log the overlay request
        log::debug!("Overlay render requested: {:?}", overlay);
        Ok(())
    }
    
    fn clear_overlays(&mut self) -> Result<()> {
        // TODO: Clear overlay buffers
        Ok(())
    }
    
    fn present(&mut self) -> Result<()> {
        let frame_start = Instant::now();
        
        // Get current surface texture
        let surface_texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Lost) => {
                // Reconfigure surface
                self.reconfigure_surface()?;
                return Ok(());
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(CCPlayerError::GpuError("Out of GPU memory".to_string()));
            }
            Err(e) => {
                log::warn!("Surface texture acquisition failed: {:?}", e);
                return Ok(());
            }
        };
        
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create command encoder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        // Render video frame
        self.pipeline.render_video(
            &self.device,
            &mut encoder,
            &view,
            &self.texture_manager,
            self.window_size,
            self.aspect_ratio,
        )?;
        
        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));
        
        // Present frame
        surface_texture.present();
        
        // Update statistics
        self.update_stats(frame_start);
        
        Ok(())
    }
    
    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        
        self.window_size = (width, height);
        self.surface_config.width = width;
        self.surface_config.height = height;
        
        self.reconfigure_surface()?;
        
        self.stats.render_width = width;
        self.stats.render_height = height;
        
        Ok(())
    }
    
    fn set_aspect_ratio(&mut self, aspect_ratio: f32) -> Result<()> {
        if aspect_ratio <= 0.0 {
            return Err(CCPlayerError::InvalidInput(
                "Aspect ratio must be positive".to_string()
            ));
        }
        
        self.aspect_ratio = aspect_ratio;
        Ok(())
    }
    
    fn screenshot(&self) -> Result<Vec<u8>> {
        // TODO: Implement screenshot functionality
        // This would involve reading back the framebuffer
        Err(CCPlayerError::NotImplemented("Screenshot not yet implemented".to_string()))
    }
}

impl WgpuRenderer {
    /// Initialize wgpu instance, device, queue, and surface
    async fn init_wgpu(
        window: &dyn Window,
    ) -> Result<(wgpu::Device, wgpu::Queue, wgpu::Surface<'static>, wgpu::SurfaceConfiguration)> {
        // Create wgpu instance with all backends
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // Create surface from window
        let surface = unsafe {
            let target = wgpu::SurfaceTargetUnsafe::from_window(window.raw_window_handle()?)
                .map_err(|e| CCPlayerError::GpuError(format!("Failed to create surface target: {:?}", e)))?;
            
            instance.create_surface_unsafe(target)
                .map_err(|e| CCPlayerError::GpuError(format!("Failed to create surface: {:?}", e)))?
        };
        
        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| CCPlayerError::GpuError("Failed to find suitable GPU adapter".to_string()))?;
        
        // Create device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("CCPlayer GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| CCPlayerError::GpuError(format!("Failed to create GPU device: {:?}", e)))?;
        
        // Get surface capabilities
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        
        // Configure surface
        let (width, height) = window.size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        surface.configure(&device, &surface_config);
        
        Ok((device, queue, surface, surface_config))
    }
    
    /// Reconfigure the surface (e.g., after resize or lost surface)
    fn reconfigure_surface(&mut self) -> Result<()> {
        self.surface.configure(&self.device, &self.surface_config);
        Ok(())
    }
    
    /// Update render statistics
    fn update_stats(&mut self, frame_start: Instant) {
        let now = Instant::now();
        let frame_time = now.duration_since(frame_start).as_secs_f32() * 1000.0;
        
        // Update frame times
        self.frame_times.push(now);
        
        // Remove old frame times (older than 1 second)
        let one_second_ago = now - std::time::Duration::from_secs(1);
        self.frame_times.retain(|&t| t > one_second_ago);
        
        // Calculate FPS
        self.stats.fps = self.frame_times.len() as f32;
        
        // Update average frame time
        if self.stats.frame_time == 0.0 {
            self.stats.frame_time = frame_time;
        } else {
            // Exponential moving average
            self.stats.frame_time = self.stats.frame_time * 0.9 + frame_time * 0.1;
        }
        
        // Check for dropped frames (frame time > 16.67ms for 60fps)
        if frame_time > 16.67 * 1.5 {
            self.stats.dropped_frames += 1;
        }
        
        self.last_frame_time = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_aspect_ratio_validation() {
        // Test aspect ratio validation logic
        // Note: Full renderer tests would require a mock window
    }
}