use wgpu::util::DeviceExt;

pub struct UiRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    texture: wgpu::Texture,
    vertex_buffer: wgpu::Buffer,
    pub show_ui: bool,
    fps_value: i32,
    gravity_value: bool,
    repulsion_value: bool,
    paused_value: bool,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct UiVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

impl UiRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        // Create texture for text - larger to fit status bar
        let texture_size = wgpu::Extent3d {
            width: 512,
            height: 64,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UI Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
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
        
        // Quads for UI elements
        // 1. Top-right FPS counter (larger for better visibility)
        // 2. Bottom status bar
        let vertices = [
            // FPS Counter Quad (top right, enlarged)
            // UV U goes from 0.0 to 0.25 to only show the text part of the texture (approx 128px width)
            // instead of the full 512px which makes it look tiny/squashed
            UiVertex { pos: [0.55, 0.95], uv: [0.0, 0.0] },
            UiVertex { pos: [0.55, 0.80], uv: [0.0, 0.5] },
            UiVertex { pos: [0.98, 0.95], uv: [0.25, 0.0] },
            UiVertex { pos: [0.98, 0.95], uv: [0.25, 0.0] },
            UiVertex { pos: [0.55, 0.80], uv: [0.0, 0.5] },
            UiVertex { pos: [0.98, 0.80], uv: [0.25, 0.5] },

            // Status Bar Quad (bottom left)
            UiVertex { pos: [-0.98, -0.85], uv: [0.0, 0.5] },
            UiVertex { pos: [-0.98, -0.95], uv: [0.0, 1.0] },
            UiVertex { pos: [0.0, -0.85], uv: [1.0, 0.5] },
            UiVertex { pos: [0.0, -0.85], uv: [1.0, 0.5] },
            UiVertex { pos: [-0.98, -0.95], uv: [0.0, 1.0] },
            UiVertex { pos: [0.0, -0.95], uv: [1.0, 1.0] },
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
            show_ui: true,
            fps_value: -1,
            gravity_value: false,
            repulsion_value: false,
            paused_value: false,
        }
    }
    
    pub fn update(&mut self, queue: &wgpu::Queue, fps: i32, gravity: bool, repulsion: bool, paused: bool) {
        if fps == self.fps_value 
            && gravity == self.gravity_value 
            && repulsion == self.repulsion_value 
            && paused == self.paused_value 
        {
            return;
        }
        
        self.fps_value = fps;
        self.gravity_value = gravity;
        self.repulsion_value = repulsion;
        self.paused_value = paused;
        
        let bytes_per_row = 512; // Adjusted for texture width
        let mut pixels = vec![0u8; 64 * bytes_per_row];
        let scale = 2; // Keep at 2 for proper rendering, quad size handles display enlargement

        let draw_char = |c: char, ox: usize, oy: usize, pixels: &mut [u8]| {
            let pattern: &[u8] = match c {
                '0' => &[0x3E, 0x51, 0x49, 0x45, 0x3E],
                '1' => &[0x00, 0x42, 0x7F, 0x40, 0x00],
                '2' => &[0x42, 0x61, 0x51, 0x49, 0x46],
                '3' => &[0x21, 0x41, 0x45, 0x4B, 0x31],
                '4' => &[0x18, 0x14, 0x12, 0x7F, 0x10],
                '5' => &[0x27, 0x45, 0x45, 0x45, 0x39],
                '6' => &[0x3C, 0x4A, 0x49, 0x49, 0x30],
                '7' => &[0x01, 0x71, 0x09, 0x05, 0x03],
                '8' => &[0x36, 0x49, 0x49, 0x49, 0x36],
                '9' => &[0x06, 0x49, 0x49, 0x29, 0x1E],
                'F' => &[0x7F, 0x09, 0x09, 0x09, 0x01],
                'P' => &[0x7F, 0x09, 0x09, 0x09, 0x06],
                'S' => &[0x46, 0x49, 0x49, 0x49, 0x31],
                'G' => &[0x3E, 0x41, 0x49, 0x51, 0x32],
                'R' => &[0x7F, 0x09, 0x19, 0x29, 0x46],
                'A' => &[0x7C, 0x12, 0x11, 0x12, 0x7C],
                'V' => &[0x1F, 0x20, 0x40, 0x20, 0x1F],
                'I' => &[0x41, 0x7F, 0x41],
                'T' => &[0x01, 0x01, 0x7F, 0x01, 0x01],
                'Y' => &[0x03, 0x04, 0x78, 0x04, 0x03],
                'M' => &[0x7F, 0x02, 0x0C, 0x02, 0x7F],
                'O' => &[0x3E, 0x41, 0x41, 0x41, 0x3E],
                'U' => &[0x3F, 0x40, 0x40, 0x40, 0x3F],
                'E' => &[0x7F, 0x49, 0x49, 0x49, 0x41],
                'C' => &[0x3E, 0x41, 0x41, 0x41, 0x22],
                'H' => &[0x7F, 0x08, 0x08, 0x08, 0x7F],
                'N' => &[0x7F, 0x04, 0x08, 0x10, 0x7F],
                'L' => &[0x7F, 0x40, 0x40, 0x40, 0x40],
                ':' => &[0x00, 0x36, 0x36, 0x00, 0x00],
                '|' => &[0x00, 0x00, 0x7F, 0x00, 0x00],
                ' ' => &[0x00, 0x00, 0x00, 0x00, 0x00],
                 _  => &[0x00, 0x00, 0x00, 0x00, 0x00],
            };
            
            for (col_idx, &col_byte) in pattern.iter().enumerate() {
                for y in 0..7 {
                    if (col_byte >> y) & 1 == 1 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = ox + col_idx * scale + sx;
                                let py = oy + y * scale + sy;
                                if px < bytes_per_row && py < 64 {
                                    pixels[py * bytes_per_row + px] = 255;
                                }
                            }
                        }
                    }
                }
            }
        };

        // Line 1: FPS (Top portion of texture)
        let s1 = format!("FPS: {}", fps);
        let mut x = 4;
        for c in s1.chars() {
            draw_char(c, x, 4, &mut pixels);
            x += 6 * scale;
        }

        // Line 2: Status Bar (Bottom portion of texture)
        let g_str = if gravity { "ON" } else { "OFF" };
        let m_str = if repulsion { "CHASE" } else { "GRAB" };
        let p_str = if paused { "PAUSED" } else { "RUNNING" };
        let s2 = format!("GRAVITY: {} | MOUSE: {} | {}", g_str, m_str, p_str);
        
        x = 4;
        for c in s2.chars() {
            draw_char(c, x, 36, &mut pixels); // Offset oy to 36 for bottom line
            x += (if c == 'I' || c == '|' { 4 } else { 6 }) * scale;
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
                rows_per_image: Some(64),
            },
            wgpu::Extent3d {
                width: 512,
                height: 64,
                depth_or_array_layers: 1,
            }
        );
    }
    
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.show_ui {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..12, 0..1); // 12 vertices for 2 quads
        }
    }
}
