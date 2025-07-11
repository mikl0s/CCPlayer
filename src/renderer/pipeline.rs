//! Render pipeline setup for video rendering
//! 
//! This module manages the GPU render pipeline, including shaders,
//! vertex buffers, and uniform buffers.

use crate::utils::error::{CCPlayerError, Result};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::texture::TextureManager;

/// Vertex data for rendering a quad
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    /// Position in normalized device coordinates
    position: [f32; 3],
    /// Texture coordinates
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x3,  // position
        1 => Float32x2,  // tex_coords
    ];
    
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Uniform buffer data for video rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct VideoUniforms {
    /// Transform matrix for aspect ratio correction
    transform: [[f32; 4]; 4],
    /// Color space conversion matrix (for YUV to RGB)
    color_matrix: [[f32; 4]; 4],
    /// Video properties (width, height, format, padding)
    video_props: [f32; 4],
}

/// Render pipeline for video rendering
pub struct RenderPipeline {
    /// Main render pipeline
    pipeline: wgpu::RenderPipeline,
    
    /// Vertex buffer for quad
    vertex_buffer: wgpu::Buffer,
    
    /// Index buffer for quad
    index_buffer: wgpu::Buffer,
    
    /// Uniform buffer
    uniform_buffer: wgpu::Buffer,
    
    /// Bind group layout
    bind_group_layout: wgpu::BindGroupLayout,
    
    /// Current bind group (recreated when textures change)
    bind_group: Option<wgpu::BindGroup>,
}

impl RenderPipeline {
    /// Create a new render pipeline
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Result<Self> {
        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Video Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/video.wgsl").into()),
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Video Bind Group Layout"),
            entries: &[
                // Uniform buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Y plane texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // U plane texture
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // V plane texture
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Video Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Video Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        
        // Create vertex buffer for a full-screen quad
        let vertices = [
            Vertex { position: [-1.0, -1.0, 0.0], tex_coords: [0.0, 1.0] }, // Bottom-left
            Vertex { position: [ 1.0, -1.0, 0.0], tex_coords: [1.0, 1.0] }, // Bottom-right
            Vertex { position: [ 1.0,  1.0, 0.0], tex_coords: [1.0, 0.0] }, // Top-right
            Vertex { position: [-1.0,  1.0, 0.0], tex_coords: [0.0, 0.0] }, // Top-left
        ];
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        // Create index buffer
        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        // Create uniform buffer with default values
        let uniforms = VideoUniforms {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            color_matrix: Self::get_yuv_to_rgb_matrix(),
            video_props: [0.0, 0.0, 0.0, 0.0],
        };
        
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        Ok(Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            bind_group_layout,
            bind_group: None,
        })
    }
    
    /// Render video frame
    pub fn render_video(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        texture_manager: &TextureManager,
        window_size: (u32, u32),
        aspect_ratio: f32,
    ) -> Result<()> {
        // Update uniforms
        self.update_uniforms(device, encoder, window_size, aspect_ratio)?;
        
        // Create bind group if needed
        if self.bind_group.is_none() {
            self.create_bind_group(device, texture_manager)?;
        }
        
        // Begin render pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Video Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        
        // Set pipeline and bind group
        render_pass.set_pipeline(&self.pipeline);
        if let Some(bind_group) = &self.bind_group {
            render_pass.set_bind_group(0, bind_group, &[]);
        }
        
        // Set vertex and index buffers
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        
        // Draw
        render_pass.draw_indexed(0..6, 0, 0..1);
        
        Ok(())
    }
    
    /// Update uniform buffer
    fn update_uniforms(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        window_size: (u32, u32),
        aspect_ratio: f32,
    ) -> Result<()> {
        // Calculate transform matrix for aspect ratio correction
        let window_aspect = window_size.0 as f32 / window_size.1 as f32;
        let scale_x = if aspect_ratio > window_aspect {
            1.0
        } else {
            aspect_ratio / window_aspect
        };
        let scale_y = if aspect_ratio > window_aspect {
            window_aspect / aspect_ratio
        } else {
            1.0
        };
        
        let uniforms = VideoUniforms {
            transform: [
                [scale_x, 0.0, 0.0, 0.0],
                [0.0, scale_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            color_matrix: Self::get_yuv_to_rgb_matrix(),
            video_props: [0.0, 0.0, 0.0, 0.0],
        };
        
        // Create staging buffer
        let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Staging Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::COPY_SRC,
        });
        
        // Update uniform buffer
        encoder.copy_buffer_to_buffer(
            &staging_buffer,
            0,
            &self.uniform_buffer,
            0,
            std::mem::size_of::<VideoUniforms>() as u64,
        );
        
        Ok(())
    }
    
    /// Create bind group for current textures
    fn create_bind_group(&mut self, device: &wgpu::Device, texture_manager: &TextureManager) -> Result<()> {
        let (y_view, u_view, v_view, sampler) = texture_manager.get_video_views()?;
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Video Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(y_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(u_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(v_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        
        self.bind_group = Some(bind_group);
        Ok(())
    }
    
    /// Get YUV to RGB color conversion matrix (BT.709)
    fn get_yuv_to_rgb_matrix() -> [[f32; 4]; 4] {
        // BT.709 YUV to RGB conversion matrix
        [
            [1.164,  0.000,  1.793, -0.973],
            [1.164, -0.213, -0.533,  0.301],
            [1.164,  2.112,  0.000, -1.133],
            [0.000,  0.000,  0.000,  1.000],
        ]
    }
    
    /// Invalidate bind group (call when textures change)
    pub fn invalidate_bind_group(&mut self) {
        self.bind_group = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vertex_layout() {
        let desc = Vertex::desc();
        assert_eq!(desc.array_stride, std::mem::size_of::<Vertex>() as u64);
    }
}