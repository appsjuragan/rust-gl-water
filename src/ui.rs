use wgpu::util::DeviceExt;

pub struct UiRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    texture: wgpu::Texture,
    vertex_buffer: wgpu::Buffer,
    pub show_fps: bool,
    fps_value: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct UiVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

impl UiRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        // Create texture for text
        let texture_size = wgpu::Extent3d {
            width: 128,
            height: 32,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm, // Single channel for alpha/intensity
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        // Bind group
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UI Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(r#"
                struct VertexInput {
                    @location(0) pos: vec2<f32>,
                    @location(1) uv: vec2<f32>,
                };
                struct VertexOutput {
                    @builtin(position) position: vec4<f32>,
                    @location(0) uv: vec2<f32>,
                };
                
                @vertex
                fn vs_main(in: VertexInput) -> VertexOutput {
                    var out: VertexOutput;
                    out.position = vec4<f32>(in.pos, 0.0, 1.0);
                    out.uv = in.uv;
                    return out;
                }
                
                @group(0) @binding(0) var t_text: texture_2d<f32>;
                @group(0) @binding(1) var s_text: sampler;
                
                @fragment
                fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                    let alpha = textureSample(t_text, s_text, in.uv).r;
                    if (alpha < 0.1) {
                        discard;
                    }
                    return vec4<f32>(1.0, 1.0, 1.0, alpha);
                }
            "#)),
        });

        // Pipeline
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("UI Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<UiVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 8, shader_location: 1 },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Disable culling to be sure
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        
        // Quad for top right
        // Screen coords: -1 to 1. Top right is (1, 1).
        // Flip U coordinates to fix mirroring if needed
        let vertices = [
            UiVertex { pos: [0.75, 0.95], uv: [1.0, 0.0] }, // Top-Left of quad, UV Right? No, flip U. Left of quad = Right of texture implies mirror.
            // Wait. If currently it IS mirrored, it means I mapped Left:0, Right:1, but it SHOWS Right-to-Left.
            // So I should map Left:1, Right:0?
            // Wait. If I want "F"(Left) to appear at Screen Left.
            // Current: Left(0.75) -> U(0).
            // Result: Mirrored. Means U(0) is appearing on Right? Or F is drawn backwards?
            // If I flip U: Left(0.75) -> U(1). Right(0.95) -> U(0).
            // Then Screen Left shows Texture Right. Screen Right shows Texture Left.
            // If texture is "F P S", then Screen shows "S P F" (backwards string, forwards letters if flipped UV?).
            
            // Let's assume the safe fix for "Mirrored" text is flipping U.
            UiVertex { pos: [0.75, 0.95], uv: [1.0, 0.0] }, 
            UiVertex { pos: [0.75, 0.85], uv: [1.0, 1.0] }, 
            UiVertex { pos: [0.95, 0.95], uv: [0.0, 0.0] }, 
            UiVertex { pos: [0.95, 0.95], uv: [0.0, 0.0] },
            UiVertex { pos: [0.75, 0.85], uv: [1.0, 1.0] },
            UiVertex { pos: [0.95, 0.85], uv: [0.0, 1.0] },
        ];
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            pipeline,
            bind_group,
            texture,
            vertex_buffer,
            show_fps: true,
            fps_value: 0,
        }
    }
    
    pub fn update(&mut self, queue: &wgpu::Queue, fps: i32) {
        if fps as u32 == self.fps_value {
            return;
        }
        self.fps_value = fps as u32;
        
        // Draw text "FPS: <fps>"
        let s = format!("FPS:{}", fps);
        let bytes_per_row = 256; // Must be multiple of 256
        let mut pixels = vec![0u8; 32 * bytes_per_row];
        
        // Simple 5x7 font rendering
        // 0: 0x3E, 0x51, 0x49, 0x45, 0x3E -> ...
        // Using a simpler approach: draw pixels for 0..9 and F, P, S, :
        
        let draw_char = |c: char, ox: usize, pixels: &mut [u8]| {
            let pattern: &[u8] = match c {
                '0' => &[0x7C, 0x44, 0x44, 0x44, 0x7C], // 5 bytes, each byte is a column (5x8)
                '1' => &[0x00, 0x44, 0x7C, 0x40, 0x00],
                '2' => &[0x64, 0x4C, 0x54, 0x54, 0x24],
                '3' => &[0x44, 0x44, 0x54, 0x54, 0x28],
                '4' => &[0x1C, 0x10, 0x74, 0x10, 0x10],
                '5' => &[0x5C, 0x54, 0x54, 0x54, 0x24],
                '6' => &[0x7C, 0x54, 0x54, 0x54, 0x24],
                '7' => &[0x44, 0x44, 0x54, 0x54, 0x7C],
                '8' => &[0x28, 0x54, 0x54, 0x54, 0x28],
                '9' => &[0x28, 0x54, 0x54, 0x54, 0x3C],
                'F' => &[0x7F, 0x49, 0x49, 0x49, 0x41],
                'P' => &[0x7F, 0x49, 0x49, 0x49, 0x30],
                'S' => &[0x26, 0x49, 0x49, 0x49, 0x32],
                ':' => &[0x00, 0x36, 0x36, 0x00, 0x00],
                 _  => &[0x00, 0x00, 0x00, 0x00, 0x00],
            };
            
            for (col_idx, &col_byte) in pattern.iter().enumerate() {
                for y in 0..7 {
                    if (col_byte >> y) & 1 == 1 {
                        let px = ox + col_idx;
                        let py = 10 + y; // Offset Y
                        if px < 128 && py < 32 {
                             pixels[py * bytes_per_row + px] = 255;
                        }
                    }
                }
            }
        };
        
        let mut x = 10;
        for c in s.chars() {
            draw_char(c, x, &mut pixels);
            x += 7; // Spacing
        }
        
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row as u32),
                rows_per_image: Some(32),
            },
            wgpu::Extent3d {
                width: 128,
                height: 32,
                depth_or_array_layers: 1,
            }
        );
    }
    
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.show_fps {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..6, 0..1);
        }
    }
}
