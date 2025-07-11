## FEATURE:

Build a lightweight GPU-accelerated desktop Chromecast receiver window that embeds OpenCast (or similar functionality), supports video/audio casting from mobile devices, and includes interactive controls.

- Borderless, resizable window.
- Moveable via ALT + Left Mouse Drag.
- Mouse wheel controls system or app volume up/down.
- OpenCast support bundled or fetched during first-run setup.
- Window rendering and video playback at high framerate (60 FPS+).
- GPU-accelerated rendering.
- Built with multi-platform in mind (Windows 11 for v1.0, portable to Linux/macOS).
- Single-file installer or self-contained launcher preferred.

Choose the most efficient framework or engine for the task (e.g., Tauri + WGPU, SDL2 + Electron, etc.).

## EXAMPLES:

Create an `examples/` folder with the following sample apps:

1. `draggable_window.rs` – Window moveable via ALT + Left Click.
2. `volume_control.rs` – Mouse scroll wheel mapped to system or media volume.
3. `webview_embed.rs` – Embed OpenCast UI into a native window using GPU-accelerated WebView.
4. `window_frame.rs` – Example of borderless, resizable window with GPU canvas.

## DOCUMENTATION:

Reference the following resources as part of your build:

- OpenCast: https://github.com/stestagg/OpenCast
- Webview libraries: Tauri (https://tauri.app/), Wry, or Electron.
- GPU abstraction layer: WGPU (https://wgpu.rs/) for Rust or similar for other languages.
- Input handling: egui, winit, SDL2, or OS-native APIs.
- Context7 MCP server documentation (for future extensibility)

## OTHER CONSIDERATIONS:

- Design for clean separation of UI controls and rendering logic.
- Ensure audio support works reliably across platforms (consider fallback options).
- Use permissive or compatible open source licenses (MIT, Apache 2.0 preferred).
- Use OpenCast as-is if it can be sandboxed and fetched dynamically — otherwise include minimal compatible functionality.
- Avoid hardcoding paths; use config file or CLI args if needed.
- Avoid common AI assistant bugs like skipping event loops or not correctly embedding web UIs inside GPU renderers.

This setup will serve as the foundation for an advanced desktop Chromecast receiver framework (v1.0 targeting Windows 11, v2.0 expanding to Linux/macOS with broader 3D framework compatibility including Metal, OpenGL, and DirectX).
