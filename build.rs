//! Build script for CCPlayer
//!
//! This script handles:
//! - FFmpeg library detection and configuration
//! - Platform-specific setup
//! - Feature detection
//! - Automatic FFmpeg download (optional)

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    
    // Detect target platform
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    
    println!("Building for {} on {}", target_os, target_arch);
    
    // Configure FFmpeg
    match configure_ffmpeg(&target_os) {
        Ok(_) => println!("FFmpeg configuration successful"),
        Err(e) => {
            eprintln!("Warning: FFmpeg configuration failed: {}", e);
            eprintln!("Attempting automatic FFmpeg setup...");
            
            if let Err(e) = setup_ffmpeg_automatically(&target_os, &target_arch) {
                panic!("Failed to setup FFmpeg: {}. Please install FFmpeg manually.", e);
            }
        }
    }
    
    // Platform-specific configuration
    match target_os.as_str() {
        "windows" => configure_windows(),
        "macos" => configure_macos(),
        "linux" => configure_linux(),
        _ => println!("cargo:warning=Unsupported platform: {}", target_os),
    }
    
    // Feature detection
    detect_features();
}

/// Configure FFmpeg libraries
fn configure_ffmpeg(target_os: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Check if FFmpeg is available via pkg-config
    if pkg_config::probe_library("libavcodec").is_ok()
        && pkg_config::probe_library("libavformat").is_ok()
        && pkg_config::probe_library("libavutil").is_ok()
        && pkg_config::probe_library("libswscale").is_ok()
    {
        println!("Found FFmpeg via pkg-config");
        return Ok(());
    }
    
    // Check environment variable
    if let Ok(ffmpeg_dir) = env::var("FFMPEG_DIR") {
        let ffmpeg_path = PathBuf::from(ffmpeg_dir);
        
        if ffmpeg_path.exists() {
            configure_ffmpeg_from_path(&ffmpeg_path, target_os)?;
            return Ok(());
        }
    }
    
    // Platform-specific default locations
    let default_paths = get_default_ffmpeg_paths(target_os);
    
    for path in default_paths {
        if path.exists() {
            println!("Found FFmpeg at: {}", path.display());
            configure_ffmpeg_from_path(&path, target_os)?;
            return Ok(());
        }
    }
    
    Err("FFmpeg not found".into())
}

/// Configure FFmpeg from a specific path
fn configure_ffmpeg_from_path(path: &Path, target_os: &str) -> Result<(), Box<dyn std::error::Error>> {
    let include_path = path.join("include");
    let lib_path = path.join("lib");
    
    if !include_path.exists() || !lib_path.exists() {
        return Err("Invalid FFmpeg directory structure".into());
    }
    
    // Set include path
    println!("cargo:include={}", include_path.display());
    
    // Set library search path
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    
    // Link libraries
    let libraries = ["avcodec", "avformat", "avutil", "swscale", "avdevice"];
    
    for lib in &libraries {
        match target_os {
            "windows" => {
                // On Windows, we might need to link against .lib files
                println!("cargo:rustc-link-lib={}", lib);
            }
            _ => {
                // On Unix-like systems
                println!("cargo:rustc-link-lib={}", lib);
            }
        }
    }
    
    // Windows-specific: Copy DLLs to output directory
    if target_os == "windows" {
        copy_ffmpeg_dlls(path)?;
    }
    
    Ok(())
}

/// Get default FFmpeg installation paths for each platform
fn get_default_ffmpeg_paths(target_os: &str) -> Vec<PathBuf> {
    match target_os {
        "windows" => vec![
            PathBuf::from("C:\\ffmpeg"),
            PathBuf::from("C:\\Program Files\\ffmpeg"),
            PathBuf::from("C:\\tools\\ffmpeg"),
            env::current_dir().unwrap().join("ffmpeg"),
        ],
        "macos" => vec![
            PathBuf::from("/usr/local"),
            PathBuf::from("/opt/homebrew"),
            PathBuf::from("/opt/local"), // MacPorts
        ],
        "linux" => vec![
            PathBuf::from("/usr"),
            PathBuf::from("/usr/local"),
            PathBuf::from("/opt/ffmpeg"),
        ],
        _ => vec![],
    }
}

/// Automatically download and setup FFmpeg
fn setup_ffmpeg_automatically(target_os: &str, target_arch: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=Automatic FFmpeg setup is not yet implemented");
    println!("cargo:warning=Please install FFmpeg manually:");
    
    match target_os {
        "windows" => {
            println!("cargo:warning=  1. Download from: https://www.gyan.dev/ffmpeg/builds/");
            println!("cargo:warning=  2. Extract to C:\\ffmpeg");
            println!("cargo:warning=  3. Set FFMPEG_DIR=C:\\ffmpeg");
        }
        "macos" => {
            println!("cargo:warning=  Run: brew install ffmpeg");
        }
        "linux" => {
            println!("cargo:warning=  Ubuntu/Debian: sudo apt install libavcodec-dev libavformat-dev libavutil-dev libswscale-dev");
            println!("cargo:warning=  Fedora: sudo dnf install ffmpeg-devel");
            println!("cargo:warning=  Arch: sudo pacman -S ffmpeg");
        }
        _ => {}
    }
    
    Err("Manual FFmpeg installation required".into())
}

/// Copy FFmpeg DLLs to output directory on Windows
fn copy_ffmpeg_dlls(ffmpeg_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let bin_path = ffmpeg_path.join("bin");
    
    if !bin_path.exists() {
        return Ok(());
    }
    
    let target_dir = Path::new(&out_dir).ancestors().nth(3).unwrap();
    
    let dlls = [
        "avcodec-*.dll",
        "avformat-*.dll",
        "avutil-*.dll",
        "swscale-*.dll",
        "avdevice-*.dll",
    ];
    
    for pattern in &dlls {
        let glob_pattern = bin_path.join(pattern).to_string_lossy().to_string();
        
        for entry in glob::glob(&glob_pattern).unwrap() {
            if let Ok(dll_path) = entry {
                let dll_name = dll_path.file_name().unwrap();
                let dest = target_dir.join(dll_name);
                
                if !dest.exists() {
                    std::fs::copy(&dll_path, &dest)?;
                    println!("Copied {} to output directory", dll_name.to_string_lossy());
                }
            }
        }
    }
    
    Ok(())
}

/// Windows-specific configuration
fn configure_windows() {
    // Link Windows libraries
    println!("cargo:rustc-link-lib=user32");
    println!("cargo:rustc-link-lib=dwmapi");
    println!("cargo:rustc-link-lib=shell32");
    
    // Enable Windows subsystem for release builds
    if env::var("PROFILE").unwrap() == "release" {
        println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
        println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
    }
}

/// macOS-specific configuration
fn configure_macos() {
    // Link macOS frameworks
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=CoreGraphics");
    println!("cargo:rustc-link-lib=framework=Metal");
    println!("cargo:rustc-link-lib=framework=MetalKit");
}

/// Linux-specific configuration
fn configure_linux() {
    // Link X11 libraries if available
    if pkg_config::probe_library("x11").is_ok() {
        println!("cargo:rustc-link-lib=X11");
    }
    
    // Link Vulkan if available
    if pkg_config::probe_library("vulkan").is_ok() {
        println!("cargo:rustc-cfg=feature=\"vulkan\"");
    }
}

/// Detect available features
fn detect_features() {
    // Check for hardware acceleration support
    detect_hw_acceleration();
    
    // Check for specific codec support
    detect_codec_support();
}

/// Detect hardware acceleration capabilities
fn detect_hw_acceleration() {
    // Check for NVENC (NVIDIA)
    if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
        if check_nvidia_gpu() {
            println!("cargo:rustc-cfg=feature=\"nvenc\"");
        }
    }
    
    // Check for VideoToolbox (macOS)
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-cfg=feature=\"videotoolbox\"");
    }
    
    // Check for VAAPI (Linux)
    if cfg!(target_os = "linux") {
        if pkg_config::probe_library("libva").is_ok() {
            println!("cargo:rustc-cfg=feature=\"vaapi\"");
        }
    }
}

/// Check if NVIDIA GPU is available
fn check_nvidia_gpu() -> bool {
    // Simple check - look for nvidia-smi
    Command::new("nvidia-smi")
        .arg("--query-gpu=name")
        .arg("--format=csv,noheader")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Detect codec support
fn detect_codec_support() {
    // This would ideally check FFmpeg's configuration
    // For now, we assume standard codecs are available
    let standard_codecs = ["h264", "h265", "vp9", "av1", "aac", "mp3", "opus"];
    
    for codec in &standard_codecs {
        println!("cargo:rustc-cfg=codec=\"{}\"", codec);
    }
}

/// Download FFmpeg binaries (not implemented - placeholder)
#[allow(dead_code)]
fn download_ffmpeg(target_os: &str, target_arch: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let download_url = match (target_os, target_arch) {
        ("windows", "x86_64") => "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
        ("windows", "x86") => "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
        _ => return Err("Automatic download not supported for this platform".into()),
    };
    
    println!("cargo:warning=Would download FFmpeg from: {}", download_url);
    
    // This is where we would implement actual download logic
    // For now, return an error to force manual installation
    Err("Automatic download not implemented".into())
}

// Module to handle pkg-config when it's not available
#[cfg(not(feature = "pkg-config"))]
mod pkg_config {
    pub struct Library;
    
    impl Library {
        pub fn new() -> Self {
            Library
        }
    }
    
    pub fn probe_library(_name: &str) -> Result<Library, Box<dyn std::error::Error>> {
        Err("pkg-config not available".into())
    }
}