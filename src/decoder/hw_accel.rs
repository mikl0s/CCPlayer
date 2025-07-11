//! Hardware acceleration support for video decoding
//! 
//! Provides platform-specific hardware acceleration implementations
//! for Windows (DXVA2/D3D11VA), macOS (VideoToolbox), and Linux (VAAPI).

use crate::decoder::{HwAccelMethod, MediaInfo, VideoStreamInfo};
use crate::utils::error::{CCPlayerError, Result};
use ffmpeg_next as ffmpeg;
use std::ffi::CString;

/// Hardware accelerator trait
pub trait HardwareAccelerator: Send + Sync {
    /// Get the acceleration method
    fn method(&self) -> HwAccelMethod;
    
    /// Configure FFmpeg context for hardware acceleration
    fn configure_context(&self, context: &mut ffmpeg::codec::context::Context) -> Result<()>;
    
    /// Check if this accelerator supports the given codec
    fn supports_codec(&self, codec_id: ffmpeg::codec::Id) -> bool;
    
    /// Get the hardware pixel format
    fn get_hw_pixel_format(&self) -> ffmpeg::format::Pixel;
}

/// Hardware acceleration configuration
#[derive(Debug, Clone)]
pub struct HwAccelConfig {
    /// Acceleration method to use
    pub method: HwAccelMethod,
    
    /// Device name or index (if applicable)
    pub device: Option<String>,
    
    /// Additional options
    pub options: Vec<(String, String)>,
}

impl HwAccelConfig {
    /// Detect the best hardware acceleration method for the current platform
    pub fn detect_best_method(media_info: &MediaInfo) -> Result<Option<Self>> {
        // Get the primary video stream
        let video_stream = media_info.video_streams.first()
            .ok_or_else(|| CCPlayerError::decoder_error("No video stream found"))?;
        
        // Determine codec ID from codec name
        let codec_id = codec_name_to_id(&video_stream.codec)?;
        
        // Try platform-specific methods
        #[cfg(target_os = "windows")]
        {
            // Try D3D11VA first (newer, better performance)
            if Self::check_d3d11va_support(codec_id) {
                return Ok(Some(Self {
                    method: HwAccelMethod::D3d11va,
                    device: None,
                    options: vec![],
                }));
            }
            
            // Fall back to DXVA2
            if Self::check_dxva2_support(codec_id) {
                return Ok(Some(Self {
                    method: HwAccelMethod::Dxva2,
                    device: None,
                    options: vec![],
                }));
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            if Self::check_videotoolbox_support(codec_id) {
                return Ok(Some(Self {
                    method: HwAccelMethod::VideoToolbox,
                    device: None,
                    options: vec![],
                }));
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            if Self::check_vaapi_support(codec_id) {
                return Ok(Some(Self {
                    method: HwAccelMethod::Vaapi,
                    device: Some("/dev/dri/renderD128".to_string()),
                    options: vec![],
                }));
            }
        }
        
        // Check for NVIDIA support (cross-platform)
        if Self::check_nvdec_support(codec_id) {
            return Ok(Some(Self {
                method: HwAccelMethod::Nvdec,
                device: None,
                options: vec![],
            }));
        }
        
        Ok(None)
    }
    
    #[cfg(target_os = "windows")]
    fn check_d3d11va_support(codec_id: ffmpeg::codec::Id) -> bool {
        // Check if D3D11VA is available for this codec
        matches!(codec_id, 
            ffmpeg::codec::Id::H264 |
            ffmpeg::codec::Id::HEVC |
            ffmpeg::codec::Id::VP9 |
            ffmpeg::codec::Id::AV1
        )
    }
    
    #[cfg(target_os = "windows")]
    fn check_dxva2_support(codec_id: ffmpeg::codec::Id) -> bool {
        // Check if DXVA2 is available for this codec
        matches!(codec_id,
            ffmpeg::codec::Id::H264 |
            ffmpeg::codec::Id::HEVC |
            ffmpeg::codec::Id::VP9
        )
    }
    
    #[cfg(target_os = "macos")]
    fn check_videotoolbox_support(codec_id: ffmpeg::codec::Id) -> bool {
        // Check if VideoToolbox is available for this codec
        matches!(codec_id,
            ffmpeg::codec::Id::H264 |
            ffmpeg::codec::Id::HEVC
        )
    }
    
    #[cfg(target_os = "linux")]
    fn check_vaapi_support(codec_id: ffmpeg::codec::Id) -> bool {
        // Check if VAAPI is available
        // This is a simplified check - in production you'd verify device availability
        std::path::Path::new("/dev/dri/renderD128").exists() &&
        matches!(codec_id,
            ffmpeg::codec::Id::H264 |
            ffmpeg::codec::Id::HEVC |
            ffmpeg::codec::Id::VP9 |
            ffmpeg::codec::Id::AV1
        )
    }
    
    fn check_nvdec_support(codec_id: ffmpeg::codec::Id) -> bool {
        // Check if NVDEC is available (simplified check)
        // In production, you'd check for NVIDIA driver and GPU presence
        matches!(codec_id,
            ffmpeg::codec::Id::H264 |
            ffmpeg::codec::Id::HEVC |
            ffmpeg::codec::Id::VP9 |
            ffmpeg::codec::Id::AV1
        )
    }
}

/// Create a hardware accelerator based on configuration
pub fn create_accelerator(config: HwAccelConfig) -> Result<Box<dyn HardwareAccelerator>> {
    match config.method {
        #[cfg(target_os = "windows")]
        HwAccelMethod::D3d11va => Ok(Box::new(D3d11vaAccelerator::new(config)?)),
        
        #[cfg(target_os = "windows")]
        HwAccelMethod::Dxva2 => Ok(Box::new(Dxva2Accelerator::new(config)?)),
        
        #[cfg(target_os = "macos")]
        HwAccelMethod::VideoToolbox => Ok(Box::new(VideoToolboxAccelerator::new(config)?)),
        
        #[cfg(target_os = "linux")]
        HwAccelMethod::Vaapi => Ok(Box::new(VaapiAccelerator::new(config)?)),
        
        HwAccelMethod::Nvdec => Ok(Box::new(NvdecAccelerator::new(config)?)),
        
        _ => Err(CCPlayerError::decoder_error(
            format!("Hardware acceleration method {:?} not supported on this platform", config.method)
        )),
    }
}

/// Windows D3D11VA accelerator
#[cfg(target_os = "windows")]
struct D3d11vaAccelerator {
    config: HwAccelConfig,
}

#[cfg(target_os = "windows")]
impl D3d11vaAccelerator {
    fn new(config: HwAccelConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

#[cfg(target_os = "windows")]
impl HardwareAccelerator for D3d11vaAccelerator {
    fn method(&self) -> HwAccelMethod {
        HwAccelMethod::D3d11va
    }
    
    fn configure_context(&self, context: &mut ffmpeg::codec::context::Context) -> Result<()> {
        // Set hardware acceleration
        unsafe {
            let hw_config = ffmpeg_sys_next::avcodec_get_hw_config(
                context.as_ptr() as *const _,
                0
            );
            
            if !hw_config.is_null() {
                (*context.as_mut_ptr()).hw_device_ctx = std::ptr::null_mut(); // Will be set by FFmpeg
                (*context.as_mut_ptr()).pix_fmt = ffmpeg_sys_next::AVPixelFormat::AV_PIX_FMT_D3D11;
            }
        }
        
        Ok(())
    }
    
    fn supports_codec(&self, codec_id: ffmpeg::codec::Id) -> bool {
        matches!(codec_id,
            ffmpeg::codec::Id::H264 |
            ffmpeg::codec::Id::HEVC |
            ffmpeg::codec::Id::VP9 |
            ffmpeg::codec::Id::AV1
        )
    }
    
    fn get_hw_pixel_format(&self) -> ffmpeg::format::Pixel {
        ffmpeg::format::Pixel::D3D11
    }
}

/// Windows DXVA2 accelerator
#[cfg(target_os = "windows")]
struct Dxva2Accelerator {
    config: HwAccelConfig,
}

#[cfg(target_os = "windows")]
impl Dxva2Accelerator {
    fn new(config: HwAccelConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

#[cfg(target_os = "windows")]
impl HardwareAccelerator for Dxva2Accelerator {
    fn method(&self) -> HwAccelMethod {
        HwAccelMethod::Dxva2
    }
    
    fn configure_context(&self, context: &mut ffmpeg::codec::context::Context) -> Result<()> {
        // Set hardware acceleration for DXVA2
        unsafe {
            (*context.as_mut_ptr()).hw_device_ctx = std::ptr::null_mut();
            (*context.as_mut_ptr()).pix_fmt = ffmpeg_sys_next::AVPixelFormat::AV_PIX_FMT_DXVA2_VLD;
        }
        
        Ok(())
    }
    
    fn supports_codec(&self, codec_id: ffmpeg::codec::Id) -> bool {
        matches!(codec_id,
            ffmpeg::codec::Id::H264 |
            ffmpeg::codec::Id::HEVC |
            ffmpeg::codec::Id::VP9
        )
    }
    
    fn get_hw_pixel_format(&self) -> ffmpeg::format::Pixel {
        ffmpeg::format::Pixel::DXVA2VLD
    }
}

/// macOS VideoToolbox accelerator
#[cfg(target_os = "macos")]
struct VideoToolboxAccelerator {
    config: HwAccelConfig,
}

#[cfg(target_os = "macos")]
impl VideoToolboxAccelerator {
    fn new(config: HwAccelConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

#[cfg(target_os = "macos")]
impl HardwareAccelerator for VideoToolboxAccelerator {
    fn method(&self) -> HwAccelMethod {
        HwAccelMethod::VideoToolbox
    }
    
    fn configure_context(&self, context: &mut ffmpeg::codec::context::Context) -> Result<()> {
        // Set hardware acceleration for VideoToolbox
        unsafe {
            (*context.as_mut_ptr()).hw_device_ctx = std::ptr::null_mut();
            (*context.as_mut_ptr()).pix_fmt = ffmpeg_sys_next::AVPixelFormat::AV_PIX_FMT_VIDEOTOOLBOX;
        }
        
        Ok(())
    }
    
    fn supports_codec(&self, codec_id: ffmpeg::codec::Id) -> bool {
        matches!(codec_id,
            ffmpeg::codec::Id::H264 |
            ffmpeg::codec::Id::HEVC
        )
    }
    
    fn get_hw_pixel_format(&self) -> ffmpeg::format::Pixel {
        ffmpeg::format::Pixel::VIDEOTOOLBOX
    }
}

/// Linux VAAPI accelerator
#[cfg(target_os = "linux")]
struct VaapiAccelerator {
    config: HwAccelConfig,
}

#[cfg(target_os = "linux")]
impl VaapiAccelerator {
    fn new(config: HwAccelConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

#[cfg(target_os = "linux")]
impl HardwareAccelerator for VaapiAccelerator {
    fn method(&self) -> HwAccelMethod {
        HwAccelMethod::Vaapi
    }
    
    fn configure_context(&self, context: &mut ffmpeg::codec::context::Context) -> Result<()> {
        // Set hardware acceleration for VAAPI
        unsafe {
            // Set device if specified
            if let Some(device) = &self.config.device {
                let device_cstr = CString::new(device.as_str())?;
                ffmpeg_sys_next::av_hwdevice_ctx_create(
                    &mut (*context.as_mut_ptr()).hw_device_ctx,
                    ffmpeg_sys_next::AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI,
                    device_cstr.as_ptr(),
                    std::ptr::null_mut(),
                    0
                );
            }
            
            (*context.as_mut_ptr()).pix_fmt = ffmpeg_sys_next::AVPixelFormat::AV_PIX_FMT_VAAPI;
        }
        
        Ok(())
    }
    
    fn supports_codec(&self, codec_id: ffmpeg::codec::Id) -> bool {
        matches!(codec_id,
            ffmpeg::codec::Id::H264 |
            ffmpeg::codec::Id::HEVC |
            ffmpeg::codec::Id::VP9 |
            ffmpeg::codec::Id::AV1
        )
    }
    
    fn get_hw_pixel_format(&self) -> ffmpeg::format::Pixel {
        ffmpeg::format::Pixel::VAAPI
    }
}

/// NVIDIA NVDEC accelerator (cross-platform)
struct NvdecAccelerator {
    config: HwAccelConfig,
}

impl NvdecAccelerator {
    fn new(config: HwAccelConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

impl HardwareAccelerator for NvdecAccelerator {
    fn method(&self) -> HwAccelMethod {
        HwAccelMethod::Nvdec
    }
    
    fn configure_context(&self, context: &mut ffmpeg::codec::context::Context) -> Result<()> {
        // Set hardware acceleration for NVDEC
        unsafe {
            ffmpeg_sys_next::av_hwdevice_ctx_create(
                &mut (*context.as_mut_ptr()).hw_device_ctx,
                ffmpeg_sys_next::AVHWDeviceType::AV_HWDEVICE_TYPE_CUDA,
                std::ptr::null(),
                std::ptr::null_mut(),
                0
            );
            
            (*context.as_mut_ptr()).pix_fmt = ffmpeg_sys_next::AVPixelFormat::AV_PIX_FMT_CUDA;
        }
        
        Ok(())
    }
    
    fn supports_codec(&self, codec_id: ffmpeg::codec::Id) -> bool {
        matches!(codec_id,
            ffmpeg::codec::Id::H264 |
            ffmpeg::codec::Id::HEVC |
            ffmpeg::codec::Id::VP9 |
            ffmpeg::codec::Id::AV1
        )
    }
    
    fn get_hw_pixel_format(&self) -> ffmpeg::format::Pixel {
        ffmpeg::format::Pixel::CUDA
    }
}

/// Helper function to convert codec name to FFmpeg codec ID
fn codec_name_to_id(codec_name: &str) -> Result<ffmpeg::codec::Id> {
    match codec_name.to_lowercase().as_str() {
        "h264" | "avc" => Ok(ffmpeg::codec::Id::H264),
        "h265" | "hevc" => Ok(ffmpeg::codec::Id::HEVC),
        "vp9" => Ok(ffmpeg::codec::Id::VP9),
        "av1" => Ok(ffmpeg::codec::Id::AV1),
        "vp8" => Ok(ffmpeg::codec::Id::VP8),
        "mpeg4" => Ok(ffmpeg::codec::Id::MPEG4),
        "mpeg2video" => Ok(ffmpeg::codec::Id::MPEG2VIDEO),
        _ => Err(CCPlayerError::decoder_error(format!("Unknown codec: {}", codec_name))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_codec_name_to_id() {
        assert_eq!(codec_name_to_id("h264").unwrap(), ffmpeg::codec::Id::H264);
        assert_eq!(codec_name_to_id("H264").unwrap(), ffmpeg::codec::Id::H264);
        assert_eq!(codec_name_to_id("avc").unwrap(), ffmpeg::codec::Id::H264);
        assert_eq!(codec_name_to_id("hevc").unwrap(), ffmpeg::codec::Id::HEVC);
        assert_eq!(codec_name_to_id("vp9").unwrap(), ffmpeg::codec::Id::VP9);
        assert_eq!(codec_name_to_id("av1").unwrap(), ffmpeg::codec::Id::AV1);
        assert!(codec_name_to_id("unknown_codec").is_err());
    }
    
    #[test]
    fn test_hw_accel_config_creation() {
        let config = HwAccelConfig {
            method: HwAccelMethod::None,
            device: None,
            options: vec![],
        };
        
        assert_eq!(config.method, HwAccelMethod::None);
        assert!(config.device.is_none());
        assert!(config.options.is_empty());
    }
}