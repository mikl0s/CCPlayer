# CCPlayer Planning & Architecture

## Project Overview
CCPlayer is a minimalist, high-performance media player built with Rust, focusing on smooth video playback and efficient resource usage.

## Architecture Overview

### Core Components

1. **Window Management** (`src/window/`)
   - Custom borderless window with draggable area
   - Minimize/maximize/close controls
   - Volume overlay
   - Uses winit for cross-platform window management

2. **Renderer** (`src/renderer/`)
   - wgpu-based GPU rendering pipeline
   - Video frame presentation
   - Overlay compositing (volume, controls)
   - Efficient texture management

3. **Decoder** (`src/decoder/`)
   - FFmpeg integration via rusty_ffmpeg
   - Hardware acceleration support
   - Frame buffering and synchronization
   - Support for common codecs (H.264, H.265, VP9, AV1)

4. **Audio** (`src/audio/`)
   - cpal for cross-platform audio output
   - Audio synchronization with video
   - Volume control with smooth transitions

5. **Player Controller** (`src/player/`)
   - Playback state management
   - A/V synchronization
   - User input handling
   - File loading and metadata extraction

6. **Utilities** (`src/utils/`)
   - Error handling with custom error types
   - Configuration management
   - Logging utilities
   - Common helper functions

## Technology Stack

- **Language**: Rust (latest stable)
- **Window Management**: winit
- **Graphics**: wgpu
- **Video Decoding**: rusty_ffmpeg (FFmpeg bindings)
- **Audio**: cpal
- **Async Runtime**: tokio
- **Error Handling**: anyhow + thiserror
- **Configuration**: serde + toml
- **Logging**: env_logger + log

## Design Principles

1. **Performance First**
   - Zero-copy where possible
   - Efficient memory usage
   - Hardware acceleration by default

2. **Minimal Dependencies**
   - Only essential external crates
   - Prefer standard library solutions
   - Avoid heavy frameworks

3. **Clean Architecture**
   - Clear separation of concerns
   - Well-defined module boundaries
   - Dependency injection for testability

4. **User Experience**
   - Smooth, responsive playback
   - Minimal UI that gets out of the way
   - Intuitive controls

## Code Style Guidelines

### Module Organization
- Each module should have a clear, single responsibility
- Use `mod.rs` to export public APIs
- Keep implementation details private
- Maximum file size: 500 lines

### Error Handling
- Use `thiserror` for custom error types
- Use `anyhow::Result` for application-level errors
- Provide meaningful error messages
- Log errors appropriately

### Documentation
- Every public function needs a docstring
- Use Google-style docstrings
- Document non-obvious implementation details
- Add inline comments for complex logic

### Testing
- Unit tests in `/tests` directory
- Test public APIs thoroughly
- Include edge cases and error scenarios
- Mock external dependencies

## Development Phases

### Wave 1: Core Infrastructure (Current)
- Basic project structure
- Module interfaces and traits
- Error handling setup
- Configuration management

### Wave 2: Window & Rendering
- Borderless window implementation
- Basic wgpu rendering pipeline
- Window controls (min/max/close)
- Drag functionality

### Wave 3: Video Decoding
- FFmpeg integration
- Basic codec support
- Frame extraction and buffering
- Hardware acceleration setup

### Wave 4: Audio Integration
- Audio output via cpal
- A/V synchronization
- Volume control

### Wave 5: Player Logic
- Playback control
- File loading
- State management
- User input handling

### Wave 6: Polish & Features
- Performance optimization
- Additional codec support
- Keyboard shortcuts
- File association

## Module Interfaces

### Window Module
```rust
trait Window {
    fn new(config: WindowConfig) -> Result<Self>;
    fn show(&mut self) -> Result<()>;
    fn hide(&mut self) -> Result<()>;
    fn set_title(&mut self, title: &str) -> Result<()>;
    fn handle_events(&mut self) -> Result<Vec<WindowEvent>>;
}
```

### Renderer Module
```rust
trait Renderer {
    fn new(window: &Window) -> Result<Self>;
    fn render_frame(&mut self, frame: VideoFrame) -> Result<()>;
    fn render_overlay(&mut self, overlay: Overlay) -> Result<()>;
    fn present(&mut self) -> Result<()>;
}
```

### Decoder Module
```rust
trait Decoder {
    fn new() -> Result<Self>;
    fn open_file(&mut self, path: &Path) -> Result<MediaInfo>;
    fn decode_frame(&mut self) -> Result<Option<VideoFrame>>;
    fn seek(&mut self, timestamp: Duration) -> Result<()>;
}
```

### Audio Module
```rust
trait AudioOutput {
    fn new() -> Result<Self>;
    fn play(&mut self, samples: &[f32]) -> Result<()>;
    fn pause(&mut self) -> Result<()>;
    fn set_volume(&mut self, volume: f32) -> Result<()>;
}
```

### Player Module
```rust
trait Player {
    fn new(window: Window, renderer: Renderer, decoder: Decoder, audio: AudioOutput) -> Result<Self>;
    fn load_file(&mut self, path: &Path) -> Result<()>;
    fn play(&mut self) -> Result<()>;
    fn pause(&mut self) -> Result<()>;
    fn seek(&mut self, position: Duration) -> Result<()>;
}
```

## Performance Targets

- **Startup Time**: < 500ms
- **Memory Usage**: < 100MB for UI (excluding video buffers)
- **CPU Usage**: < 5% during idle
- **Frame Drops**: < 0.1% during normal playback
- **Seek Time**: < 100ms for local files

## Security Considerations

- Validate all file inputs
- Sandbox decoder operations
- Limit resource usage
- No network access in core player
- Safe FFmpeg API usage