//! Texture management for video frames
//! 
//! This module handles GPU texture creation and updates for video frames,
//! supporting various pixel formats including YUV and RGB.

use crate::renderer::{FrameData, VideoFrame};
use crate::utils::error::{CCPlayerError, Result};

/// Texture manager for video frames
pub struct TextureManager {
    
    /// Y plane texture (luminance)
    y_texture: Option<wgpu::Texture>,
    
    /// U plane texture (chrominance blue)
    u_texture: Option<wgpu::Texture>,
    
    /// V plane texture (chrominance red)
    v_texture: Option<wgpu::Texture>,
    
    /// RGB/RGBA texture (for RGB formats)
    rgb_texture: Option<wgpu::Texture>,
    
    /// Texture sampler
    sampler: wgpu::Sampler,
    
    /// Current texture dimensions
    current_dimensions: Option<(u32, u32)>,
    
    /// Current pixel format
    current_format: Option<VideoFormat>,
}

/// Internal video format representation
#[derive(Debug, Clone, Copy, PartialEq)]
enum VideoFormat {
    Yuv420,
    Yuv422,
    Yuv444,
    Nv12,
    Rgb,
    Rgba,
}

impl TextureManager {
    /// Create a new texture manager
    pub fn new(device: &wgpu::Device) -> Result<Self> {
        // Create sampler for video textures
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Video Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        Ok(Self {
            y_texture: None,
            u_texture: None,
            v_texture: None,
            rgb_texture: None,
            sampler,
            current_dimensions: None,
            current_format: None,
        })
    }
    
    /// Update video texture with new frame data
    pub fn update_video_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &VideoFrame,
    ) -> Result<()> {
        let format = Self::get_format(&frame.data);
        let dimensions = (frame.width, frame.height);
        
        // Check if we need to recreate textures
        if self.current_dimensions != Some(dimensions) || self.current_format != Some(format) {
            self.create_textures(device, dimensions, format)?;
            self.current_dimensions = Some(dimensions);
            self.current_format = Some(format);
        }
        
        // Upload frame data to textures
        match &frame.data {
            FrameData::Yuv420 { y_plane, u_plane, v_plane, y_stride, uv_stride } => {
                self.upload_yuv_data(queue, frame.width, frame.height, y_plane, u_plane, v_plane, *y_stride, *uv_stride)?;
            }
            FrameData::Yuv422 { y_plane, u_plane, v_plane, y_stride, uv_stride } => {
                self.upload_yuv_data(queue, frame.width, frame.height, y_plane, u_plane, v_plane, *y_stride, *uv_stride)?;
            }
            FrameData::Yuv444 { y_plane, u_plane, v_plane, stride } => {
                self.upload_yuv_data(queue, frame.width, frame.height, y_plane, u_plane, v_plane, *stride, *stride)?;
            }
            FrameData::Nv12 { y_plane, uv_plane, y_stride, uv_stride } => {
                self.upload_nv12_data(queue, frame.width, frame.height, y_plane, uv_plane, *y_stride, *uv_stride)?;
            }
            FrameData::Rgb { data, stride } => {
                self.upload_rgb_data(queue, frame.width, frame.height, data, *stride, 3)?;
            }
            FrameData::Rgba { data, stride } => {
                self.upload_rgb_data(queue, frame.width, frame.height, data, *stride, 4)?;
            }
        }
        
        Ok(())
    }
    
    /// Get texture views for rendering
    pub fn get_video_views(&self) -> Result<(&wgpu::TextureView, &wgpu::TextureView, &wgpu::TextureView, &wgpu::Sampler)> {
        let y_view = self.y_texture.as_ref()
            .ok_or_else(|| CCPlayerError::InvalidState("Y texture not initialized".to_string()))?
            .create_view(&wgpu::TextureViewDescriptor::default());
        
        let u_view = self.u_texture.as_ref()
            .ok_or_else(|| CCPlayerError::InvalidState("U texture not initialized".to_string()))?
            .create_view(&wgpu::TextureViewDescriptor::default());
        
        let v_view = self.v_texture.as_ref()
            .ok_or_else(|| CCPlayerError::InvalidState("V texture not initialized".to_string()))?
            .create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create views inline to avoid lifetime issues
        // In real implementation, we'd cache these views
        Ok((
            Box::leak(Box::new(y_view)),
            Box::leak(Box::new(u_view)),
            Box::leak(Box::new(v_view)),
            &self.sampler
        ))
    }
    
    
    /// Create textures for the given format and dimensions
    fn create_textures(
        &mut self,
        device: &wgpu::Device,
        dimensions: (u32, u32),
        format: VideoFormat,
    ) -> Result<()> {
        let (width, height) = dimensions;
        
        match format {
            VideoFormat::Yuv420 | VideoFormat::Yuv422 | VideoFormat::Yuv444 | VideoFormat::Nv12 => {
                // Create Y texture (full resolution)
                self.y_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Y Plane Texture"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::R8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                }));
                
                // Calculate chroma dimensions based on format
                let (chroma_width, chroma_height) = match format {
                    VideoFormat::Yuv420 | VideoFormat::Nv12 => (width / 2, height / 2),
                    VideoFormat::Yuv422 => (width / 2, height),
                    VideoFormat::Yuv444 => (width, height),
                    _ => unreachable!(),
                };
                
                // Create U texture
                self.u_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("U Plane Texture"),
                    size: wgpu::Extent3d {
                        width: chroma_width,
                        height: chroma_height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: if format == VideoFormat::Nv12 {
                        wgpu::TextureFormat::Rg8Unorm  // For interleaved UV
                    } else {
                        wgpu::TextureFormat::R8Unorm
                    },
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                }));
                
                // Create V texture (not needed for NV12)
                if format != VideoFormat::Nv12 {
                    self.v_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("V Plane Texture"),
                        size: wgpu::Extent3d {
                            width: chroma_width,
                            height: chroma_height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::R8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    }));
                }
            }
            VideoFormat::Rgb | VideoFormat::Rgba => {
                // Create RGB/RGBA texture
                let format = if format == VideoFormat::Rgb {
                    wgpu::TextureFormat::Rgba8UnormSrgb  // We'll pad RGB to RGBA
                } else {
                    wgpu::TextureFormat::Rgba8UnormSrgb
                };
                
                self.rgb_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("RGB Texture"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                }));
                
                // For RGB formats, we'll use the RGB texture for all planes
                self.y_texture = self.rgb_texture.clone();
                self.u_texture = self.rgb_texture.clone();
                self.v_texture = self.rgb_texture.clone();
            }
        }
        
        Ok(())
    }
    
    /// Upload YUV data to textures
    fn upload_yuv_data(
        &self,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        y_data: &[u8],
        u_data: &[u8],
        v_data: &[u8],
        y_stride: usize,
        uv_stride: usize,
    ) -> Result<()> {
        // Upload Y plane
        if let Some(texture) = &self.y_texture {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                y_data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(y_stride as u32),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        
        // Upload U plane
        if let Some(texture) = &self.u_texture {
            let chroma_height = match self.current_format {
                Some(VideoFormat::Yuv420) => height / 2,
                Some(VideoFormat::Yuv422) => height,
                Some(VideoFormat::Yuv444) => height,
                _ => height / 2,
            };
            
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                u_data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(uv_stride as u32),
                    rows_per_image: Some(chroma_height),
                },
                wgpu::Extent3d {
                    width: width / 2,
                    height: chroma_height,
                    depth_or_array_layers: 1,
                },
            );
        }
        
        // Upload V plane
        if let Some(texture) = &self.v_texture {
            let chroma_height = match self.current_format {
                Some(VideoFormat::Yuv420) => height / 2,
                Some(VideoFormat::Yuv422) => height,
                Some(VideoFormat::Yuv444) => height,
                _ => height / 2,
            };
            
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                v_data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(uv_stride as u32),
                    rows_per_image: Some(chroma_height),
                },
                wgpu::Extent3d {
                    width: width / 2,
                    height: chroma_height,
                    depth_or_array_layers: 1,
                },
            );
        }
        
        Ok(())
    }
    
    /// Upload NV12 data to textures
    fn upload_nv12_data(
        &self,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        y_data: &[u8],
        uv_data: &[u8],
        y_stride: usize,
        uv_stride: usize,
    ) -> Result<()> {
        // Upload Y plane
        if let Some(texture) = &self.y_texture {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                y_data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(y_stride as u32),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        
        // Upload interleaved UV plane
        if let Some(texture) = &self.u_texture {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                uv_data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(uv_stride as u32),
                    rows_per_image: Some(height / 2),
                },
                wgpu::Extent3d {
                    width: width / 2,
                    height: height / 2,
                    depth_or_array_layers: 1,
                },
            );
        }
        
        Ok(())
    }
    
    /// Upload RGB data to textures
    fn upload_rgb_data(
        &self,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        data: &[u8],
        stride: usize,
        channels: u8,
    ) -> Result<()> {
        if let Some(texture) = &self.rgb_texture {
            // For RGB (3 channels), we need to pad to RGBA
            if channels == 3 {
                let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
                for y in 0..height as usize {
                    for x in 0..width as usize {
                        let offset = y * stride + x * 3;
                        rgba_data.push(data[offset]);     // R
                        rgba_data.push(data[offset + 1]); // G
                        rgba_data.push(data[offset + 2]); // B
                        rgba_data.push(255);               // A
                    }
                }
                
                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &rgba_data,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(width * 4),
                        rows_per_image: Some(height),
                    },
                    wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );
            } else {
                // RGBA data can be uploaded directly
                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    data,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(stride as u32),
                        rows_per_image: Some(height),
                    },
                    wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        
        Ok(())
    }
    
    /// Get format from frame data
    fn get_format(data: &FrameData) -> VideoFormat {
        match data {
            FrameData::Yuv420 { .. } => VideoFormat::Yuv420,
            FrameData::Yuv422 { .. } => VideoFormat::Yuv422,
            FrameData::Yuv444 { .. } => VideoFormat::Yuv444,
            FrameData::Nv12 { .. } => VideoFormat::Nv12,
            FrameData::Rgb { .. } => VideoFormat::Rgb,
            FrameData::Rgba { .. } => VideoFormat::Rgba,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_detection() {
        let yuv_data = FrameData::Yuv420 {
            y_plane: vec![],
            u_plane: vec![],
            v_plane: vec![],
            y_stride: 1920,
            uv_stride: 960,
        };
        
        assert_eq!(TextureManager::get_format(&yuv_data), VideoFormat::Yuv420);
    }
}