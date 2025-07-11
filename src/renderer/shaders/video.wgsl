// Video rendering shader for CCPlayer
// Supports YUV to RGB conversion and aspect ratio correction

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct Uniforms {
    // Transform matrix for aspect ratio correction
    transform: mat4x4<f32>,
    // Color space conversion matrix
    color_matrix: mat4x4<f32>,
    // Video properties: x=width, y=height, z=format, w=reserved
    video_props: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var y_texture: texture_2d<f32>;

@group(0) @binding(2)
var u_texture: texture_2d<f32>;

@group(0) @binding(3)
var v_texture: texture_2d<f32>;

@group(0) @binding(4)
var video_sampler: sampler;

// Vertex shader
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Apply aspect ratio correction
    let transformed_pos = uniforms.transform * vec4<f32>(input.position, 1.0);
    output.clip_position = vec4<f32>(transformed_pos.xy, 0.0, 1.0);
    output.tex_coords = input.tex_coords;
    
    return output;
}

// Fragment shader
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample YUV textures
    let y = textureSample(y_texture, video_sampler, input.tex_coords).r;
    let u = textureSample(u_texture, video_sampler, input.tex_coords).r;
    let v = textureSample(v_texture, video_sampler, input.tex_coords).r;
    
    // Apply YUV to RGB conversion
    // Using BT.709 color space conversion matrix
    let yuv = vec4<f32>(y, u, v, 1.0);
    let rgb = uniforms.color_matrix * yuv;
    
    // Clamp values to valid range
    let clamped_rgb = clamp(rgb.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    
    return vec4<f32>(clamped_rgb, 1.0);
}

// Alternative fragment shader for RGB/RGBA formats
@fragment
fn fs_main_rgb(input: VertexOutput) -> @location(0) vec4<f32> {
    // For RGB formats, we use the y_texture which contains the RGB data
    return textureSample(y_texture, video_sampler, input.tex_coords);
}

// Fragment shader for NV12 format (interleaved UV)
@fragment
fn fs_main_nv12(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample Y texture
    let y = textureSample(y_texture, video_sampler, input.tex_coords).r;
    
    // Sample interleaved UV texture (RG channels contain U and V)
    let uv = textureSample(u_texture, video_sampler, input.tex_coords).rg;
    
    // Apply YUV to RGB conversion
    let yuv = vec4<f32>(y, uv.x, uv.y, 1.0);
    let rgb = uniforms.color_matrix * yuv;
    
    // Clamp values to valid range
    let clamped_rgb = clamp(rgb.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    
    return vec4<f32>(clamped_rgb, 1.0);
}

// Overlay rendering support (for future use)
struct OverlayVertex {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct OverlayVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_overlay(input: OverlayVertex) -> OverlayVertexOutput {
    var output: OverlayVertexOutput;
    
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coords = input.tex_coords;
    output.color = input.color;
    
    return output;
}

@fragment
fn fs_overlay(input: OverlayVertexOutput) -> @location(0) vec4<f32> {
    // Simple colored overlay
    return input.color;
}

// HDR support preparation (for future implementation)
fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.0031308);
    let a = vec3<f32>(0.055);
    let gamma = vec3<f32>(2.4);
    
    let lower = linear * 12.92;
    let higher = pow(linear, vec3<f32>(1.0 / gamma.x)) * (1.0 + a.x) - a;
    
    return select(higher, lower, linear <= cutoff);
}

fn srgb_to_linear(srgb: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.04045);
    let a = vec3<f32>(0.055);
    let gamma = vec3<f32>(2.4);
    
    let lower = srgb / 12.92;
    let higher = pow((srgb + a) / (1.0 + a.x), gamma);
    
    return select(higher, lower, srgb <= cutoff);
}

// Tone mapping for HDR content (Reinhard operator)
fn tone_map_reinhard(color: vec3<f32>) -> vec3<f32> {
    return color / (color + vec3<f32>(1.0));
}

// ACES filmic tone mapping
fn tone_map_aces(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}