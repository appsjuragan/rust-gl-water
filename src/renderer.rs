//! Renderer module - handles scene rendering with water, caustics, and pool


use glam::Vec3;
use wgpu::util::DeviceExt;
use crate::physics::ObjectState;

use crate::camera::{Camera, CameraUniform};
use crate::shaders::{caustics::caustics_shader, pool::pool_shader, water_render::water_shader, sphere::sphere_shader};
use crate::water::Water;

/// Resolution for caustics texture
pub const CAUSTICS_TEXTURE_SIZE: u32 = 512;

/// Common uniforms shared across shaders
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CommonUniforms {
    pub pool_height: f32,
    pub wall_height: f32,
    pub pool_size: [f32; 2],
    pub light_dir: [f32; 4],
    pub sphere_centers: [[f32; 4]; 5],
    pub sphere_radius: f32,
    pub time: f32,
    pub shape_type: i32,
    pub texture_type: i32,
    pub object_count: i32,
    pub pool_shape: i32,
    pub _padding: [f32; 2],
    pub light_color: [f32; 4],
}

impl Default for CommonUniforms {
    fn default() -> Self {
        Self {
            pool_height: 1.0,
            wall_height: 0.4,
            pool_size: [1.0, 1.0],
            light_dir: [-0.577, 0.577, 0.577, 0.0],
            sphere_centers: [[0.0; 4]; 5],
            sphere_radius: 0.25,
            time: 0.0,
            shape_type: 0,
            texture_type: 0,
            object_count: 1,
            pool_shape: 0,
            _padding: [0.0; 2],
            light_color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Vertex for water surface mesh
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaterVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

impl WaterVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WaterVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Vertex for pool cube
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PoolVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl PoolVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x2,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PoolVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Main renderer
pub struct Renderer {
    // Uniform buffers
    camera_uniform_buffer: wgpu::Buffer,
    common_uniform_buffer: wgpu::Buffer,
    camera_uniform: CameraUniform,
    pub common_uniform: CommonUniforms,

    // Textures
    _tile_texture: wgpu::Texture,
    tile_texture_view: wgpu::TextureView,
    tile_sampler: wgpu::Sampler,
    _sky_texture: wgpu::Texture,
    sky_texture_view: wgpu::TextureView,
    sky_sampler: wgpu::Sampler,
    _caustic_texture: wgpu::Texture,
    caustic_texture_view: wgpu::TextureView,
    caustic_sampler: wgpu::Sampler,

    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,

    // Pipelines
    water_pipeline: wgpu::RenderPipeline,
    pool_pipeline: wgpu::RenderPipeline,
    caustics_pipeline: wgpu::RenderPipeline,
    sphere_pipeline: wgpu::RenderPipeline,

    // Bind groups
    camera_bind_group: wgpu::BindGroup,
    water_texture_bind_group_layout: wgpu::BindGroupLayout,
    caustics_bind_group_layout: wgpu::BindGroupLayout,

    // Meshes
    water_vertex_buffer: wgpu::Buffer,
    water_index_buffer: wgpu::Buffer,
    water_index_count: u32,

    pool_vertex_buffer: wgpu::Buffer,
    pool_index_buffer: wgpu::Buffer,
    pool_index_count: u32,

    sphere_vertex_buffer: wgpu::Buffer,
    sphere_index_buffer: wgpu::Buffer,
    sphere_index_count: u32,

    // Light direction
    pub light_dir: Vec3,

    // Pool dimensions
    pub pool_width: f32,
    pub pool_length: f32,
    pub pool_height: f32,
    pub wall_height: f32,

    // Sphere (duck) state
    pub sphere_radius: f32,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let light_dir = Vec3::new(-1.0, 1.0, 1.0).normalize();

        // Create uniform buffers
        let camera_uniform = CameraUniform::new();
        let camera_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let common_uniform = CommonUniforms {
            light_dir: [light_dir.x, light_dir.y, light_dir.z, 0.0],
            ..Default::default()
        };
        let common_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Common Uniform Buffer"),
            contents: bytemuck::cast_slice(&[common_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create default textures
        let (tile_texture, tile_texture_view, tile_sampler) =
            Self::create_default_texture(device, queue, "Tile", [220, 220, 220, 255]);
        let (sky_texture, sky_texture_view, sky_sampler) =
            Self::create_default_texture(device, queue, "Sky", [135, 206, 235, 255]);

        // Create caustics render target
        let caustic_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Caustics Texture"),
            size: wgpu::Extent3d {
                width: CAUSTICS_TEXTURE_SIZE,
                height: CAUSTICS_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let caustic_texture_view = caustic_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let caustic_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Caustic Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Create depth texture
        let (depth_texture, depth_texture_view) = Self::create_depth_texture(device, width, height);

        // Create camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: common_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Water texture bind group layout
        let water_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Water Texture Bind Group Layout"),
                entries: &[
                    // Water texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Tile texture
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Caustic texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Sky texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Caustics bind group layout
        let caustics_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Caustics Bind Group Layout"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Create water pipeline
        let water_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Water Shader"),
            source: wgpu::ShaderSource::Wgsl(water_shader().into()),
        });

        let water_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Water Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &water_texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let water_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Water Pipeline"),
            layout: Some(&water_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &water_shader_module,
                entry_point: "vs_main",
                buffers: &[WaterVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &water_shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Double-sided
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create pool pipeline
        let pool_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pool Shader"),
            source: wgpu::ShaderSource::Wgsl(pool_shader().into()),
        });

        let pool_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pool Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &water_texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pool_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pool Pipeline"),
            layout: Some(&pool_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &pool_shader_module,
                entry_point: "vs_main",
                buffers: &[PoolVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &pool_shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front), // Back face (inside of cube)
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create caustics pipeline
        let caustics_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Caustics Shader"),
            source: wgpu::ShaderSource::Wgsl(caustics_shader().into()),
        });

        let caustics_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Caustics Pipeline Layout"),
                bind_group_layouts: &[&caustics_bind_group_layout],
                push_constant_ranges: &[],
            });

        let caustics_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Caustics Pipeline"),
            layout: Some(&caustics_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &caustics_shader_module,
                entry_point: "vs_main",
                buffers: &[WaterVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &caustics_shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::SrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create sphere pipeline
        let sphere_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sphere Shader"),
            source: wgpu::ShaderSource::Wgsl(sphere_shader().into()),
        });

        // Reuse pool layout (camera + water texture bind groups)
        let sphere_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sphere Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &water_texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let sphere_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sphere Pipeline"),
            layout: Some(&sphere_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sphere_shader_module,
                entry_point: "vs_main",
                buffers: &[PoolVertex::desc()], // Reuse PoolVertex layout
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sphere_shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create water mesh (grid)
        let (water_vertices, water_indices) = Self::create_water_mesh(100);
        let water_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Water Vertex Buffer"),
            contents: bytemuck::cast_slice(&water_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let water_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Water Index Buffer"),
            contents: bytemuck::cast_slice(&water_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create pool cube mesh
        let (pool_vertices, pool_indices) = Self::create_cube_mesh();
        let pool_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pool Vertex Buffer"),
            contents: bytemuck::cast_slice(&pool_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let pool_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pool Index Buffer"),
            contents: bytemuck::cast_slice(&pool_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create sphere mesh
        let (sphere_vertices, sphere_indices) = Self::create_sphere_mesh(1.0, 32, 32);
        let sphere_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sphere Vertex Buffer"),
            contents: bytemuck::cast_slice(&sphere_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let sphere_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sphere Index Buffer"),
            contents: bytemuck::cast_slice(&sphere_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            camera_uniform_buffer,
            common_uniform_buffer,
            camera_uniform,
            common_uniform,
            _tile_texture: tile_texture,
            tile_texture_view,
            tile_sampler,
            _sky_texture: sky_texture,
            sky_texture_view,
            sky_sampler,
            _caustic_texture: caustic_texture,
            caustic_texture_view,
            caustic_sampler,
            depth_texture,
            depth_texture_view,
            water_pipeline,
            pool_pipeline,
            caustics_pipeline,
            sphere_pipeline,
            camera_bind_group,
            water_texture_bind_group_layout,
            caustics_bind_group_layout,
            water_vertex_buffer,
            water_index_buffer,
            water_index_count: water_indices.len() as u32,
            pool_vertex_buffer,
            pool_index_buffer,
            pool_index_count: pool_indices.len() as u32,
            sphere_vertex_buffer,
            sphere_index_buffer,
            sphere_index_count: sphere_indices.len() as u32,
            light_dir,
            pool_width: 2.0,
            pool_length: 2.0,
            pool_height: 1.0,
            wall_height: 0.4,
            sphere_radius: 0.25,
        }
    }

    pub fn update_object_mesh(&mut self, device: &wgpu::Device, shape: &str) {
        let (vertices, indices) = match shape {
            "Cube" => {
                let (mut v, i) = Self::create_cube_mesh();
                // Normalize cube to fit in unit sphere (scale by 1/sqrt(3))
                let scale = 1.0 / 3.0f32.sqrt();
                for vert in &mut v {
                    vert.position[0] *= scale;
                    vert.position[1] *= scale;
                    vert.position[2] *= scale;
                }
                (v, i)
            },
            "Torus" => Self::create_torus_mesh(0.7, 0.3, 32, 16),
            "Tetrahedron" => Self::create_tetrahedron_mesh(1.0 / 3.0f32.sqrt()),
            _ => Self::create_sphere_mesh(1.0, 32, 32), // Default to sphere
        };

        self.sphere_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Object Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.sphere_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Object Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.sphere_index_count = indices.len() as u32;
    }

    fn create_default_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        color: [u8; 4],
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
        let size = 64u32;
        let mut data = vec![0u8; (size * size * 4) as usize];
        
        // Create a simple tile pattern
        for y in 0..size {
            for x in 0..size {
                let idx = ((y * size + x) * 4) as usize;
                let is_edge = x % 16 < 2 || y % 16 < 2;
                if is_edge {
                    data[idx] = (color[0] as f32 * 0.7) as u8;
                    data[idx + 1] = (color[1] as f32 * 0.7) as u8;
                    data[idx + 2] = (color[2] as f32 * 0.7) as u8;
                } else {
                    data[idx] = color[0];
                    data[idx + 1] = color[1];
                    data[idx + 2] = color[2];
                }
                data[idx + 3] = color[3];
            }
        }

        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &data,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("{} Sampler", label)),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        (texture, view, sampler)
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn create_water_mesh(resolution: u32) -> (Vec<WaterVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for y in 0..=resolution {
            for x in 0..=resolution {
                let u = x as f32 / resolution as f32;
                let v = y as f32 / resolution as f32;
                vertices.push(WaterVertex {
                    position: [u * 2.0 - 1.0, 0.0, v * 2.0 - 1.0],
                    uv: [u, v],
                });
            }
        }

        for y in 0..resolution {
            for x in 0..resolution {
                let i = y * (resolution + 1) + x;
                indices.push(i);
                indices.push(i + resolution + 1);
                indices.push(i + 1);
                indices.push(i + 1);
                indices.push(i + resolution + 1);
                indices.push(i + resolution + 2);
            }
        }

        (vertices, indices)
    }

    fn create_cube_mesh() -> (Vec<PoolVertex>, Vec<u32>) {
        // Unit cube centered at origin
        let vertices = vec![
            // Front face
            PoolVertex { position: [-1.0, -1.0,  1.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0] },
            PoolVertex { position: [ 1.0, -1.0,  1.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0] },
            PoolVertex { position: [ 1.0,  1.0,  1.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0] },
            PoolVertex { position: [-1.0,  1.0,  1.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0] },
            // Back face
            PoolVertex { position: [-1.0, -1.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [1.0, 0.0] },
            PoolVertex { position: [-1.0,  1.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [1.0, 1.0] },
            PoolVertex { position: [ 1.0,  1.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [0.0, 1.0] },
            PoolVertex { position: [ 1.0, -1.0, -1.0], normal: [0.0, 0.0, -1.0], uv: [0.0, 0.0] },
            // Top face
            PoolVertex { position: [-1.0,  1.0, -1.0], normal: [0.0, 1.0, 0.0], uv: [0.0, 1.0] },
            PoolVertex { position: [-1.0,  1.0,  1.0], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0] },
            PoolVertex { position: [ 1.0,  1.0,  1.0], normal: [0.0, 1.0, 0.0], uv: [1.0, 0.0] },
            PoolVertex { position: [ 1.0,  1.0, -1.0], normal: [0.0, 1.0, 0.0], uv: [1.0, 1.0] },
            // Bottom face
            PoolVertex { position: [-1.0, -1.0, -1.0], normal: [0.0, -1.0, 0.0], uv: [0.0, 0.0] },
            PoolVertex { position: [ 1.0, -1.0, -1.0], normal: [0.0, -1.0, 0.0], uv: [1.0, 0.0] },
            PoolVertex { position: [ 1.0, -1.0,  1.0], normal: [0.0, -1.0, 0.0], uv: [1.0, 1.0] },
            PoolVertex { position: [-1.0, -1.0,  1.0], normal: [0.0, -1.0, 0.0], uv: [0.0, 1.0] },
            // Right face
            PoolVertex { position: [ 1.0, -1.0, -1.0], normal: [1.0, 0.0, 0.0], uv: [1.0, 0.0] },
            PoolVertex { position: [ 1.0,  1.0, -1.0], normal: [1.0, 0.0, 0.0], uv: [1.0, 1.0] },
            PoolVertex { position: [ 1.0,  1.0,  1.0], normal: [1.0, 0.0, 0.0], uv: [0.0, 1.0] },
            PoolVertex { position: [ 1.0, -1.0,  1.0], normal: [1.0, 0.0, 0.0], uv: [0.0, 0.0] },
            // Left face
            PoolVertex { position: [-1.0, -1.0, -1.0], normal: [-1.0, 0.0, 0.0], uv: [0.0, 0.0] },
            PoolVertex { position: [-1.0, -1.0,  1.0], normal: [-1.0, 0.0, 0.0], uv: [1.0, 0.0] },
            PoolVertex { position: [-1.0,  1.0,  1.0], normal: [-1.0, 0.0, 0.0], uv: [1.0, 1.0] },
            PoolVertex { position: [-1.0,  1.0, -1.0], normal: [-1.0, 0.0, 0.0], uv: [0.0, 1.0] },
        ];

        let indices: Vec<u32> = vec![
            0, 1, 2, 0, 2, 3,       // Front
            4, 5, 6, 4, 6, 7,       // Back
            8, 9, 10, 8, 10, 11,    // Top
            12, 13, 14, 12, 14, 15, // Bottom
            16, 17, 18, 16, 18, 19, // Right
            20, 21, 22, 20, 22, 23, // Left
        ];

        (vertices, indices)
    }

    fn create_sphere_mesh(radius: f32, lat_segments: u32, lon_segments: u32) -> (Vec<PoolVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for y in 0..=lat_segments {
            for x in 0..=lon_segments {
                let u = x as f32 / lon_segments as f32;
                let v = y as f32 / lat_segments as f32;
                
                let theta = u * std::f32::consts::PI * 2.0;
                let phi = v * std::f32::consts::PI;
                
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();
                let sin_theta = theta.sin();
                let cos_theta = theta.cos();
                
                let nx = cos_theta * sin_phi;
                let ny = cos_phi;
                let nz = sin_theta * sin_phi;
                
                vertices.push(PoolVertex {
                    position: [nx * radius, ny * radius, nz * radius],
                    normal: [nx, ny, nz],
                    uv: [u, v],
                });
            }
        }

        for y in 0..lat_segments {
            for x in 0..lon_segments {
                let first = (y * (lon_segments + 1)) + x;
                let second = first + lon_segments + 1;
                
                indices.push(first);
                indices.push(first + 1);
                indices.push(second);
                
                indices.push(second);
                indices.push(first + 1);
                indices.push(second + 1);
            }
        }

        (vertices, indices)
    }


    fn create_torus_mesh(radius: f32, tube_radius: f32, radial_segments: u32, tubular_segments: u32) -> (Vec<PoolVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for j in 0..=radial_segments {
            for i in 0..=tubular_segments {
                let u = i as f32 / tubular_segments as f32;
                let v = j as f32 / radial_segments as f32;

                let theta = u * std::f32::consts::PI * 2.0;
                let phi = v * std::f32::consts::PI * 2.0;

                let cos_theta = theta.cos();
                let sin_theta = theta.sin();
                let cos_phi = phi.cos();
                let sin_phi = phi.sin();

                let x = (radius + tube_radius * cos_phi) * cos_theta;
                let y = tube_radius * sin_phi;
                let z = (radius + tube_radius * cos_phi) * sin_theta;

                let nx = cos_phi * cos_theta;
                let ny = sin_phi;
                let nz = cos_phi * sin_theta;

                vertices.push(PoolVertex {
                    position: [x, y, z],
                    normal: [nx, ny, nz],
                    uv: [u, v],
                });
            }
        }

        for j in 0..radial_segments {
            for i in 0..tubular_segments {
                let first = (j * (tubular_segments + 1)) + i;
                let second = first + tubular_segments + 1;

                indices.push(first);
                indices.push(second);
                indices.push(first + 1);

                indices.push(second);
                indices.push(second + 1);
                indices.push(first + 1);
            }
        }

        (vertices, indices)
    }

    fn create_tetrahedron_mesh(size: f32) -> (Vec<PoolVertex>, Vec<u32>) {
         // Define vertices for a simple unit tetrahedron
        let p0 = [ 1.0,  1.0,  1.0];
        let p1 = [-1.0, -1.0,  1.0];
        let p2 = [-1.0,  1.0, -1.0];
        let p3 = [ 1.0, -1.0, -1.0];
        
        // Scale
        let p0 = [p0[0]*size, p0[1]*size, p0[2]*size];
        let p1 = [p1[0]*size, p1[1]*size, p1[2]*size];
        let p2 = [p2[0]*size, p2[1]*size, p2[2]*size];
        let p3 = [p3[0]*size, p3[1]*size, p3[2]*size];

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        let faces = [
            (p0, p2, p1),
            (p0, p3, p2),
            (p0, p1, p3),
            (p1, p2, p3)
        ];
        
        let uvs = [[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];

        let mut idx = 0;
        for (a, b, c) in faces {
             // Calculate normal
             let v0 = glam::Vec3::from_array(a);
             let v1 = glam::Vec3::from_array(b);
             let v2 = glam::Vec3::from_array(c);
             let normal = (v1 - v0).cross(v2 - v0).normalize().to_array();
             
             vertices.push(PoolVertex { position: a, normal, uv: uvs[0] });
             vertices.push(PoolVertex { position: b, normal, uv: uvs[1] });
             vertices.push(PoolVertex { position: c, normal, uv: uvs[2] });
             
             indices.push(idx);
             indices.push(idx + 1);
             indices.push(idx + 2);
             idx += 3;
        }

        (vertices, indices)
    }

    /// Create a frustum (truncated pyramid) pool mesh - narrower at bottom
    fn create_frustum_pool_mesh(bottom_scale: f32) -> (Vec<PoolVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        // Top is at scale 1.0, bottom is at bottom_scale (e.g., 0.7)
        let top = 1.0f32;
        let bot = bottom_scale;
        
        // Bottom face (smaller square)
        let bottom_verts = [
            PoolVertex { position: [-bot, -1.0, -bot], normal: [0.0, -1.0, 0.0], uv: [0.0, 0.0] },
            PoolVertex { position: [ bot, -1.0, -bot], normal: [0.0, -1.0, 0.0], uv: [1.0, 0.0] },
            PoolVertex { position: [ bot, -1.0,  bot], normal: [0.0, -1.0, 0.0], uv: [1.0, 1.0] },
            PoolVertex { position: [-bot, -1.0,  bot], normal: [0.0, -1.0, 0.0], uv: [0.0, 1.0] },
        ];
        vertices.extend_from_slice(&bottom_verts);
        indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        
        // Four trapezoidal walls
        let wall_data = [
            // Front wall (z = positive)
            ([-top, 1.0, top], [top, 1.0, top], [bot, -1.0, bot], [-bot, -1.0, bot], [0.0, 0.0, 1.0]),
            // Back wall (z = negative)
            ([top, 1.0, -top], [-top, 1.0, -top], [-bot, -1.0, -bot], [bot, -1.0, -bot], [0.0, 0.0, -1.0]),
            // Right wall (x = positive)
            ([top, 1.0, top], [top, 1.0, -top], [bot, -1.0, -bot], [bot, -1.0, bot], [1.0, 0.0, 0.0]),
            // Left wall (x = negative)
            ([-top, 1.0, -top], [-top, 1.0, top], [-bot, -1.0, bot], [-bot, -1.0, -bot], [-1.0, 0.0, 0.0]),
        ];
        
        let mut idx = 4u32; // Start after bottom face vertices
        for (tl, tr, br, bl, normal) in wall_data {
            vertices.push(PoolVertex { position: tl, normal, uv: [0.0, 0.0] });
            vertices.push(PoolVertex { position: tr, normal, uv: [1.0, 0.0] });
            vertices.push(PoolVertex { position: br, normal, uv: [1.0, 1.0] });
            vertices.push(PoolVertex { position: bl, normal, uv: [0.0, 1.0] });
            
            indices.extend_from_slice(&[idx, idx+1, idx+2, idx, idx+2, idx+3]);
            idx += 4;
        }
        
        (vertices, indices)
    }
    
    /// Create a cylindrical pool mesh
    fn create_cylinder_pool_mesh(segments: u32) -> (Vec<PoolVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        let radius = 1.0f32;
        
        // Bottom face (circle) - center vertex + ring
        // Normal points UP (toward inside of pool, since we view from above)
        let center_idx = 0u32;
        vertices.push(PoolVertex { 
            position: [0.0, -1.0, 0.0], 
            normal: [0.0, 1.0, 0.0],  // Changed to point up (inside pool)
            uv: [0.5, 0.5] 
        });
        
        // Ring vertices for bottom face - CCW winding when viewed from above
        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::PI * 2.0;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            
            vertices.push(PoolVertex {
                position: [x, -1.0, z],
                normal: [0.0, 1.0, 0.0],  // Point up (inside pool)
                uv: [0.5 + x * 0.5, 0.5 + z * 0.5],
            });
        }
        
        // Bottom face indices - CCW winding when viewed from above (inside pool)
        for i in 0..segments {
            let curr = i + 1;
            let next = if i + 1 == segments { 1 } else { i + 2 };
            // CCW when viewed from above: center, curr, next
            indices.extend_from_slice(&[center_idx, curr, next]);
        }
        
        // Cylindrical wall - normals point INWARD (toward center)
        let wall_start = vertices.len() as u32;
        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::PI * 2.0;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            let u = i as f32 / segments as f32;
            
            // Normal points INWARD (negative of outward direction)
            let nx = -angle.cos();
            let nz = -angle.sin();
            
            // Bottom vertex of wall
            vertices.push(PoolVertex {
                position: [x, -1.0, z],
                normal: [nx, 0.0, nz],
                uv: [u, 1.0],
            });
            
            // Top vertex of wall
            vertices.push(PoolVertex {
                position: [x, 1.0, z],
                normal: [nx, 0.0, nz],
                uv: [u, 0.0],
            });
        }
        
        // Wall indices - CCW winding when viewed from inside
        for i in 0..segments {
            let base = wall_start + i * 2;
            // For inside view with inward normals:
            // Triangle 1: bottom_curr, top_curr, bottom_next (CCW from inside)
            // Triangle 2: top_curr, top_next, bottom_next (CCW from inside)
            indices.extend_from_slice(&[
                base, base + 1, base + 2,      // bottom_i, top_i, bottom_i+1
                base + 1, base + 3, base + 2,  // top_i, top_i+1, bottom_i+1
            ]);
        }
        
        (vertices, indices)
    }
    
    /// Update pool mesh based on selected shape
    pub fn update_pool_mesh(&mut self, device: &wgpu::Device, shape: &str) {
        let (vertices, indices) = match shape {
            "Cuboid" | "Cube" => Self::create_cube_mesh(),
            "Frustum" => Self::create_frustum_pool_mesh(0.7), // 70% size at bottom
            "Cylinder" => Self::create_cylinder_pool_mesh(32),
            _ => Self::create_cube_mesh(), // Default to cube
        };
        
        self.pool_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pool Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.pool_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pool Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.pool_index_count = indices.len() as u32;
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let (depth_texture, depth_texture_view) = Self::create_depth_texture(device, width, height);
        self.depth_texture = depth_texture;
        self.depth_texture_view = depth_texture_view;
    }

    pub fn update_uniforms(
        &mut self,
        queue: &wgpu::Queue,
        camera: &Camera,
        objects: &[ObjectState],
        sphere_radius: f32,
        time: f32,
        shape_type: i32,
        texture_type: i32,
        light_color: [f32; 3],
        pool_shape: i32,
    ) {
        self.camera_uniform.update(camera);
        queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        // Populate centers array
        let mut centers = [[0.0f32; 4]; 5];
        let count = objects.len().min(5);
        for i in 0..count {
            centers[i] = [objects[i].center.x, objects[i].center.y, objects[i].center.z, 0.0];
        }

        self.common_uniform.pool_height = self.pool_height;
        self.common_uniform.wall_height = self.wall_height;
        self.common_uniform.pool_size = [self.pool_width / 2.0, self.pool_length / 2.0];
        self.common_uniform.light_dir = [self.light_dir.x, self.light_dir.y, self.light_dir.z, 0.0];
        self.common_uniform.sphere_centers = centers;
        self.common_uniform.sphere_radius = sphere_radius;
        self.common_uniform.time = time;
        self.common_uniform.shape_type = shape_type;
        self.common_uniform.texture_type = texture_type;
        self.common_uniform.object_count = count as i32;
        self.common_uniform.light_color = [light_color[0], light_color[1], light_color[2], 1.0];
        self.common_uniform.pool_shape = pool_shape;

        // self.sphere_center is no longer tracked per object in Renderer struct
        self.sphere_radius = sphere_radius;

        queue.write_buffer(
            &self.common_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.common_uniform]),
        );
    }

    pub fn update_dimensions(&mut self, width: f32, length: f32, depth: f32, wall_height: f32) {
        self.pool_width = width;
        self.pool_length = length;
        self.pool_height = depth;
        self.wall_height = wall_height;
    }

    pub fn render_caustics(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        water: &Water,
    ) {
        let caustics_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Caustics Bind Group"),
            layout: &self.caustics_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.common_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(water.current_texture_view()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(water.sampler()),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Caustics Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.caustic_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.caustics_pipeline);
        pass.set_bind_group(0, &caustics_bind_group, &[]);
        pass.set_vertex_buffer(0, self.water_vertex_buffer.slice(..));
        pass.set_index_buffer(self.water_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.water_index_count, 0, 0..1);
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        water: &Water,
    ) {
        // Create texture bind group with current water texture
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Water Texture Bind Group"),
            layout: &self.water_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(water.current_texture_view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(water.sampler()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.tile_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.tile_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&self.caustic_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.caustic_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&self.sky_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&self.sky_sampler),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Render pool (walls/floor)
        pass.set_pipeline(&self.pool_pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &texture_bind_group, &[]);
        pass.set_vertex_buffer(0, self.pool_vertex_buffer.slice(..));
        pass.set_index_buffer(self.pool_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.pool_index_count, 0, 0..1);

        // Render sphere
        pass.set_pipeline(&self.sphere_pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &texture_bind_group, &[]);
        pass.set_vertex_buffer(0, self.sphere_vertex_buffer.slice(..));
        pass.set_index_buffer(self.sphere_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.sphere_index_count, 0, 0..self.common_uniform.object_count as u32);

        // Render water surface
        pass.set_pipeline(&self.water_pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &texture_bind_group, &[]);
        pass.set_vertex_buffer(0, self.water_vertex_buffer.slice(..));
        pass.set_index_buffer(self.water_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.water_index_count, 0, 0..1);
    }
}
