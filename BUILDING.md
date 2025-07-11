# Building CCPlayer

This guide provides detailed instructions for building CCPlayer on various platforms.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Platform-Specific Instructions](#platform-specific-instructions)
  - [Windows](#windows)
  - [macOS](#macos)
  - [Linux](#linux)
- [Build Configuration](#build-configuration)
- [Troubleshooting](#troubleshooting)
- [Cross-Compilation](#cross-compilation)

## Prerequisites

### Required Tools

1. **Rust Toolchain** (1.75.0 or later)
   ```bash
   # Install via rustup
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Git** for cloning the repository

3. **C/C++ Compiler**
   - Windows: MSVC (Visual Studio 2019 or later)
   - macOS: Xcode Command Line Tools
   - Linux: GCC or Clang

4. **FFmpeg Development Libraries** (5.0 or later)

## Platform-Specific Instructions

### Windows

#### Option 1: Using Pre-built FFmpeg

1. **Install Visual Studio Build Tools**
   - Download from [Visual Studio Downloads](https://visualstudio.microsoft.com/downloads/)
   - Install "Desktop development with C++" workload

2. **Download FFmpeg**
   ```powershell
   # Using PowerShell
   Invoke-WebRequest -Uri "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-full-shared.7z" -OutFile "ffmpeg.7z"
   
   # Extract (requires 7-Zip)
   7z x ffmpeg.7z
   ```

3. **Set Environment Variables**
   ```powershell
   # Add to PATH
   $env:PATH += ";C:\path\to\ffmpeg\bin"
   
   # Set FFmpeg directories
   $env:FFMPEG_DIR = "C:\path\to\ffmpeg"
   $env:PKG_CONFIG_PATH = "C:\path\to\ffmpeg\lib\pkgconfig"
   ```

4. **Build CCPlayer**
   ```powershell
   git clone https://github.com/yourusername/ccplayer.git
   cd ccplayer
   cargo build --release
   ```

#### Option 2: Using vcpkg

1. **Install vcpkg**
   ```powershell
   git clone https://github.com/Microsoft/vcpkg.git
   cd vcpkg
   .\bootstrap-vcpkg.bat
   .\vcpkg integrate install
   ```

2. **Install FFmpeg**
   ```powershell
   .\vcpkg install ffmpeg:x64-windows
   ```

3. **Build with vcpkg**
   ```powershell
   cd ccplayer
   cargo build --release
   ```

### macOS

#### Using Homebrew

1. **Install Xcode Command Line Tools**
   ```bash
   xcode-select --install
   ```

2. **Install Homebrew** (if not already installed)
   ```bash
   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
   ```

3. **Install Dependencies**
   ```bash
   brew install ffmpeg pkg-config
   ```

4. **Build CCPlayer**
   ```bash
   git clone https://github.com/yourusername/ccplayer.git
   cd ccplayer
   cargo build --release
   ```

#### Using MacPorts

1. **Install MacPorts** from [macports.org](https://www.macports.org/)

2. **Install Dependencies**
   ```bash
   sudo port install ffmpeg +universal pkgconfig
   ```

3. **Build CCPlayer**
   ```bash
   export PKG_CONFIG_PATH="/opt/local/lib/pkgconfig:$PKG_CONFIG_PATH"
   cargo build --release
   ```

### Linux

#### Ubuntu/Debian

1. **Update Package List**
   ```bash
   sudo apt update
   ```

2. **Install Build Dependencies**
   ```bash
   sudo apt install -y \
     build-essential \
     pkg-config \
     libavcodec-dev \
     libavformat-dev \
     libavutil-dev \
     libswscale-dev \
     libavdevice-dev \
     libclang-dev
   ```

3. **Build CCPlayer**
   ```bash
   git clone https://github.com/yourusername/ccplayer.git
   cd ccplayer
   cargo build --release
   ```

#### Fedora/RHEL

1. **Install Build Dependencies**
   ```bash
   sudo dnf install -y \
     gcc \
     gcc-c++ \
     pkgconfig \
     ffmpeg-devel \
     clang-devel
   ```

2. **Build CCPlayer**
   ```bash
   cargo build --release
   ```

#### Arch Linux

1. **Install Dependencies**
   ```bash
   sudo pacman -S base-devel ffmpeg pkgconf clang
   ```

2. **Build CCPlayer**
   ```bash
   cargo build --release
   ```

## Build Configuration

### Feature Flags

CCPlayer supports several compile-time features:

```bash
# Default build
cargo build --release

# Build without hardware acceleration
cargo build --release --no-default-features

# Build with only specific features
cargo build --release --no-default-features --features "audio,software-decode"

# Build with additional debug symbols
cargo build --release --features "debug-renderer"
```

### Available Features

- `default`: All standard features enabled
- `hw-accel`: Hardware video acceleration (default: on)
- `audio`: Audio playback support (default: on)
- `chromecast`: Chromecast support (default: off)
- `debug-renderer`: Additional debug overlays (default: off)

### Build Profiles

#### Release Build (Recommended)
```toml
[profile.release]
lto = true
opt-level = 3
codegen-units = 1
```

#### Debug Build
```toml
[profile.dev]
opt-level = 0
debug = true
```

#### Performance-Optimized Build
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Environment Variables

### Build-Time Variables

- `FFMPEG_DIR`: Path to FFmpeg installation
- `PKG_CONFIG_PATH`: Path to pkg-config files
- `LIBCLANG_PATH`: Path to libclang (for bindgen)

### Runtime Variables

- `RUST_LOG`: Logging level (debug, info, warn, error)
- `WGPU_BACKEND`: Force specific GPU backend (vulkan, metal, dx12, opengl)

## Troubleshooting

### Common Build Issues

#### "Could not find FFmpeg"

**Solution**: Ensure FFmpeg is properly installed and pkg-config can find it:
```bash
pkg-config --modversion libavcodec
pkg-config --modversion libavformat
```

#### "error: linker `link.exe` not found" (Windows)

**Solution**: Install Visual Studio Build Tools with C++ support.

#### "error: failed to run custom build command for `ffmpeg-sys-next`"

**Solution**: 
1. Check FFmpeg version (requires 5.0+)
2. Ensure all FFmpeg development packages are installed
3. Clear cargo cache: `cargo clean`

#### Memory issues during linking

**Solution**: Increase available memory or use mold/lld linker:
```bash
# Install mold (Linux)
sudo apt install mold

# Use mold for linking
RUSTFLAGS="-C link-arg=-fuse-ld=mold" cargo build --release
```

### Platform-Specific Issues

#### Windows: Missing DLLs

After building, ensure these DLLs are in the same directory as ccplayer.exe:
- avcodec-*.dll
- avformat-*.dll
- avutil-*.dll
- swscale-*.dll

#### macOS: Code Signing

For distribution, sign the binary:
```bash
codesign --force --sign - target/release/ccplayer
```

#### Linux: Missing Graphics Libraries

Install graphics dependencies:
```bash
# Ubuntu/Debian
sudo apt install libvulkan1 mesa-vulkan-drivers

# Fedora
sudo dnf install vulkan-loader vulkan-headers
```

## Cross-Compilation

### Windows to Linux

1. **Install Cross-Compilation Tools**
   ```bash
   rustup target add x86_64-unknown-linux-gnu
   ```

2. **Install Linux Toolchain**
   ```bash
   # Using WSL or Docker is recommended
   ```

### Linux to Windows

1. **Install MinGW**
   ```bash
   sudo apt install mingw-w64
   ```

2. **Add Windows Target**
   ```bash
   rustup target add x86_64-pc-windows-gnu
   ```

3. **Cross-Compile**
   ```bash
   cargo build --release --target x86_64-pc-windows-gnu
   ```

## Docker Build

Build using Docker for consistent environment:

```dockerfile
# Dockerfile
FROM rust:1.75

RUN apt-get update && apt-get install -y \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libswscale-dev \
    pkg-config

WORKDIR /app
COPY . .

RUN cargo build --release
```

Build command:
```bash
docker build -t ccplayer-build .
docker run --rm -v $(pwd)/target:/app/target ccplayer-build
```

## Verification

After building, verify the installation:

```bash
# Check version
./target/release/ccplayer --version

# Test with a sample video
./target/release/ccplayer test.mp4

# Check linked libraries (Linux/macOS)
ldd target/release/ccplayer  # Linux
otool -L target/release/ccplayer  # macOS
```

## Next Steps

- Read the [README](README.md) for usage instructions
- Check [CONTRIBUTING](CONTRIBUTING.md) for development guidelines
- Report issues on [GitHub Issues](https://github.com/yourusername/ccplayer/issues)