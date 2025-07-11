# CCPlayer Feature Specification

## Overview

Design and implement a lightweight, high-performance media player to display video (e.g., Opencast streams) in a borderless window with audio playback. The window should be movable by dragging (using Alt + left mouse drag) and resizable, despite having no standard window frame. Scrolling the mouse wheel while the window is focused adjusts the audio volume. The application must utilize GPU acceleration for smooth playback (targeting 60 FPS or higher) and support multiple graphics backends (DirectX, OpenGL, Metal) within one codebase for portability. Version 1.0 targets Windows 11, but the architecture should be cross-platform to support Linux and macOS in version 2.0.

## Key Requirements

- **Borderless, Draggable Window**:
  - Create a window without default chrome (no title bar or borders).
  - Allow user-initiated move and resize actions:
    - **Alt + left-click drag**: Substitutes for dragging the title bar to move the window.
    - **Window edges**: Enable resizing by dragging near the window boundaries.
- **Mouse Wheel Volume Control**:
  - Adjust playback volume when scrolling the mouse wheel over the focused window (scroll up = volume up, scroll down = volume down).
- **GPU-Accelerated Rendering**:
  - Render video frames using GPU APIs for efficiency.
  - Support multiple graphics APIs (Direct3D, OpenGL, Metal) in a single codebase.
  - Use a graphics abstraction library like [bgfx](https://github.com/bkaradzic/bgfx) to handle multiple backends (DirectX 11/12, OpenGL, Metal, Vulkan).
- **High FPS with V-Sync**:
  - Target 60 FPS playback with vsync to avoid stutter.
  - Utilize hardware decoding and efficient frame buffering for smooth video playback.
- **Audio Playback**:
  - Output audio using a cross-platform audio library (e.g., SDL2's audio subsystem or Windows-specific API).
  - Implement volume control by adjusting the audio stream volume or applying a gain.
- **Portability and Modular Design**:
  - Write code with portability in mind, abstracting platform-specific functionality (window creation, event handling).
  - Use cross-platform libraries (e.g., SDL2 for windowing/input/audio, bgfx for rendering) to minimize platform-specific code.
  - Facilitate future support for Linux and macOS in version 2.0.

## Summary

The media player is a custom video player with unconventional window controls and broad hardware support. It should deliver smooth performance (no lag at 60 FPS), packaged as a single installer or self-contained executable that fetches dependencies as needed.

---

## Examples

The `examples/` folder contains sample code to aid development:

- **`sdl_borderless_resize.cpp`**:
  - Demonstrates a borderless, resizable window using SDL2.
  - Uses `SDL_SetWindowHitTest` to designate draggable and resizable regions:
    - **Alt + left-click drag**: Marks the window interior as draggable for moving.
    - **Window edges**: Marked as resize zones for resizing.
  - Renders a solid color background for simplicity during drag/resize.
  - Tracks Alt key state and integrates with SDL's hit-testing API for smooth OS-driven drag/resize.

- **`bgfx_sdl_example.cpp`**:
  - Creates a borderless SDL window and initializes bgfx for rendering.
  - Provides bgfx with the native window handle via `SDL_GetWindowWMInfo`.
  - Sets up bgfx platform data for different platforms (e.g., HWND for Windows, Display/Window for X11).
  - Uses `bgfx::RendererType::Count` to auto-select the best backend (e.g., Direct3D on Windows, Metal on macOS).
  - Handles:
    - **Window resize events** (`SDL_WINDOWEVENT_RESIZED`): Calls `bgfx::reset` with new dimensions.
    - **Mouse wheel events**: Adjusts a volume variable (simulating audio volume control).
    - **Quit events**: Exits the render loop.
  - Clears the screen each frame (placeholder for video frame rendering).
  - Verifies bgfx setup for GPU acceleration and SDL integration, supporting multiple APIs without separate render code.

**Download Examples**: [examples.zip](#) (contains `sdl_borderless_resize.cpp` and `bgfx_sdl_example.cpp`). Right-click and "Save Link As..." if clicking doesn't download.

---

## Documentation and Resources

The following resources are useful for development:

- **BGFX (Cross-Platform Rendering Library)**:
  - [GitHub](https://github.com/bkaradzic/bgfx): Lists supported backends (DirectX 11/12, OpenGL, Metal) and platforms (Windows, macOS, Linux).
  - Official docs: Guidance on integration, renderer selection, view clearing, frame submission, and resizing.
- **SDL2 Documentation** ([SDL2 Wiki](https://wiki.libsdl.org/SDL2/)):
  - **Window Creation/Management**: Use `SDL_WINDOW_BORDERLESS` and `SDL_WINDOW_RESIZABLE` flags, and `SDL_SetWindowHitTest` for custom drag/resize regions.
  - **Event Handling**:
    - Keyboard: `SDL_KEYDOWN`/`SDL_KEYUP` for Alt key detection.
    - Mouse: `SDL_MOUSEBUTTONDOWN` for clicks, `SDL_MOUSEWHEEL` for scroll.
    - Window: `SDL_WINDOWEVENT_RESIZED` for resize events.
  - **Audio**: Use `SDL_OpenAudioDevice` or `SDL_mixer` for audio playback, and `SDL_QueueAudio` to feed decoded audio frames.
  - **Licensing**: SDL2's zlib license is permissive for commercial use.
- **FFmpeg (libavcodec/libavformat)**:
  - Official docs: Cover demuxing, decoding, and playback sync.
  - Use `libavformat` to read media files/streams and `libavcodec` to decode video/audio.
  - Key APIs: `avcodec_send_packet`, `avcodec_receive_frame`, `libswscale` for frame conversion (e.g., YUV to RGB).
  - Feed PCM audio samples to SDL's audio device.
  - **Licensing**: Use LGPL components only (no GPL codecs), dynamically link to FFmpeg libraries, and provide source/attribution to comply with LGPL.
- **Platform-Specific Windowing**:
  - **Windows (Win32)**: Simulate title-bar drag with `WM_NCLBUTTONDOWN` and `HTCAPTION` after releasing mouse capture.
  - **Linux (X11)**: Use Xlib for manual window movement if needed, though SDL's hit-test typically suffices. Avoid relative coordinate issues for smooth dragging.
  - **macOS**: SDL's hit-test uses Cocoa calls for borderless window dragging.
- **SDL Forums/Stack Overflow**: Discuss borderless window dragging techniques, including Win32 fallback (`ReleaseCapture` + `SendMessage` with `WM_NCLBUTTONDOWN`).
- **Future Video Backends**:
  - macOS: AVFoundation, VideoToolbox.
  - Linux: GStreamer, VAAPI.
  - Windows: DXVA2/D3D11VA via FFmpeg for hardware decoding.

---

## Additional Considerations

### Graphics Framework
- Use bgfx (BSD-2 licensed) for cross-platform rendering to avoid separate code paths for DirectX, OpenGL, and Metal.
- Alternative: OpenGL via SDL/GLFW for Windows/Linux, Metal/MoltenVK for macOS, but bgfx is preferred to simplify multi-backend support.
- Build system: Automate bgfx library compilation or fetch precompiled binaries.

### Video Decoding and Performance
- Decoding high-resolution video at 60 FPS can be CPU-intensive.
- Use FFmpeg's hardware decoding (DXVA2/D3D11VA on Windows, VideoToolbox on macOS) to offload work to GPU/ASIC.
- For v1.0, software decoding is acceptable for 1080p; optimize with hardware decoding in v2.0.
- Structure playback with separate decoding and rendering threads:
  - Decoding thread feeds frames to a queue.
  - Rendering thread displays frames to prevent decode latency from stalling rendering.
- Buffer a few frames ahead to handle momentary slowdowns.

### Audio-Video Synchronization
- Use audio playback as the master clock (steady hardware clock).
- Sync video frames to audio using FFmpeg's `AVFrame` PTS (presentation timestamps).
- Strategy: Compare audio playback position to video frame PTS, dropping/delaying frames as needed.
- Consult FFmpeg's video sync tutorials for implementation.

### Volume Control
- Implement scroll-wheel volume control with a software volume factor (0–100).
- Scale audio samples before enqueueing to SDL audio device or use SDL_mixer for volume control.
- Provide on-screen feedback (e.g., volume indicator) or console log.
- Ensure volume adjustments only occur when the window is focused to avoid unintended changes.
- Clamp volume values to prevent clipping or out-of-range issues.

### Borderless Window UX
- **Windows**:
  - Alt + left-click drag is not reserved by the OS; implement via SDL hit-test or Win32 `ReleaseCapture` + `WM_NCLBUTTONDOWN`.
  - Ensure Alt+F4 closes the window (handled by SDL's `SDL_QUIT` event).
- **Linux**:
  - Alt + drag may conflict with desktop environment defaults (e.g., Ubuntu's window manager).
  - Consider alternative modifiers (Ctrl+Drag, Super+Drag) or make configurable for v2.0.
  - Test SDL hit-test compatibility with window managers.
- **macOS**:
  - Use Option key for Alt-drag; test SDL hit-test with Cocoa.
  - Ensure compatibility with macOS Spaces/Mission Control.
- **Resizing**:
  - Use a 10px edge region for resizing (as in example).
  - Change cursor icon near edges using `SDL_SetCursor` (`SDL_SYSTEM_CURSOR_SIZENWSE`, etc.) for better UX.
  - Address SDL issue with cursor feedback in hit-test regions.

### Dependency Management
- **Installer Strategy**:
  - Avoid statically bundling large dependencies (e.g., FFmpeg) to reduce bloat and licensing issues.
  - Download FFmpeg LGPL-compliant DLLs during installation from official builds or a trusted source.
  - Include SDL2 (zlib license) and bgfx (BSD-2) in the installer, as they are lightweight and permissive.
  - Alternatively, app can check/download DLLs on first run, but installer-based fetching is cleaner.
- **FFmpeg Licensing**:
  - Use dynamic linking for FFmpeg to comply with LGPL.
  - Provide documentation for replacing FFmpeg DLLs.
  - Avoid GPL codecs; ensure source code availability for LGPL compliance.
- **Secure Downloads**:
  - Use HTTPS for fetching dependencies.
  - Verify sources to prevent tampering.

### Testing on Windows 11
- Test Alt + drag for reliable window movement; ensure no interference from graphics drivers or overlays.
- Verify smooth volume adjustment via scroll wheel within range.
- Confirm scroll events require window focus.
- Ensure edge-of-screen windows remain grabbable for move/resize (consider padding if needed).
- Test on high-DPI displays; ensure SDL's DPI awareness and hit-test regions scale correctly.

### Full-Screen Mode
- Not required for v1.0 but consider borderless full-screen (maximized window) as a future feature (e.g., via double-click or keybind).

### Future Features
- Structure code for extensibility:
  - Support play/pause (e.g., Space bar), seeking, and UI controls (custom-rendered or via a UI library).
  - Map input events to actions for easy addition of shortcuts.
  - Allow multiple videos/playlists in future versions (v1.0 assumes single video).

### Memory Management
- Properly shut down subsystems on exit:
  - Stop audio playback.
  - Free FFmpeg decoder contexts (`av_frame_free`, `av_packet_free`).
  - Call `bgfx::shutdown` and destroy SDL window.
- Use smart pointers/RAII in C++ to prevent memory leaks.
- Handle errors gracefully (e.g., invalid video file, missing decoder) with clear user feedback.

### Patents and Codecs
- Note patent-encumbered codecs (H.264, H.265) for wide distribution.
- Use OS native decoders (e.g., Media Foundation) to offload liability.
- For internal/open-source use, patent issues are typically not a concern.

---

## Implementation Plan
- Use C++ with:
  - **SDL2**: Windowing, input, and audio.
  - **bgfx**: GPU-accelerated rendering.
  - **FFmpeg**: Video/audio decoding.
- Manage dependencies via vcpkg, Conan, or submodules.
- Scaffold project with a main loop integrating the provided examples.
- Test thoroughly for smooth 60 FPS playback, responsive window controls, and volume adjustments.
- Structure code for future cross-platform expansion (Linux, macOS).

---

## References
1. [BGFX GitHub](https://github.com/bkaradzic/bgfx)
2. [SDL_SetWindowHitTest - SDL2 Wiki](https://wiki.libsdl.org/SDL2/SDL_SetWindowHitTest)
3. [FFmpeg License and Legal Considerations](https://www.ffmpeg.org/legal.html)

---