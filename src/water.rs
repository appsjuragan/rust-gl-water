//! Water simulation module - GPU-based wave simulation using ping-pong textures

use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::shaders::water_sim::{DROP_SHADER, NORMAL_SHADER, SPHERE_VOLUME_SHADER, UPDATE_SHADER};

/// Resolution of the water simulation texture
pub const WATER_TEXTURE_SIZE: u32 = 512;

/// Water simulation using GPU ping-pong rendering
pub struct Water {
    /// Primary render target
    _texture_a: wgpu::Texture,
    texture_a_view: wgpu::TextureView,
    /// Secondary render target (ping-pong)
    _texture_b: wgpu::Texture,
    texture_b_view: wgpu::TextureView,
    /// Currently active texture (0 = A, 1 = B)
    current: usize,

    /// Sampler for water textures
    sampler: wgpu::Sampler,

    // Simulation pipelines
    drop_pipeline: wgpu::RenderPipeline,
    update_pipeline: wgpu::RenderPipeline,
    normal_pipeline: wgpu::RenderPipeline,
    sphere_pipeline: wgpu::RenderPipeline,

    // Uniform buffers
    drop_uniform_buffer: wgpu::Buffer,
    update_uniform_buffer: wgpu::Buffer,
    normal_uniform_buffer: wgpu::Buffer,
    sphere_uniform_buffer: wgpu::Buffer,

    // Bind groups (will be swapped for ping-pong)
    // Layout
    bind_group_layout: wgpu::BindGroupLayout,

    // Pool dimensions
    pub pool_width: f32,
    pub pool_length: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DropUniforms {
    center: [f32; 2],
    radius: f32,
    strength: f32,
    pool_size: [f32; 2],
    _padding: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct UpdateUniforms {
    delta: [f32; 2],
    pool_size: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SphereVolumeUniforms {
    old_center: [f32; 4],
    new_center: [f32; 4],
    radius: f32,
    strength: f32,
    pool_size: [f32; 2],
    shape_type: i32,
    _padding: [f32; 3],
}

impl Water {
    pub fn new(device: &wgpu::Device) -> Self {
        // Create ping-pong textures
        let texture_desc = wgpu::TextureDescriptor {
            label: Some("Water Texture"),
            size: wgpu::Extent3d {
                width: WATER_TEXTURE_SIZE,
                height: WATER_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        };

        let texture_a = device.create_texture(&texture_desc);
        let texture_b = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Water Texture B"),
            ..texture_desc
        });

        let texture_a_view = texture_a.create_view(&wgpu::TextureViewDescriptor::default());
        let texture_b_view = texture_b.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Water Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create uniform buffers
        let pool_width = 2.0f32;
        let pool_length = 2.0f32;

        let drop_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Drop Uniforms"),
            contents: bytemuck::cast_slice(&[DropUniforms {
                center: [0.0, 0.0],
                radius: 0.03,
                strength: 0.01,
                pool_size: [pool_width, pool_length],
                _padding: [0.0, 0.0],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let delta = 1.0 / WATER_TEXTURE_SIZE as f32;
        let update_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Update Uniforms"),
            contents: bytemuck::cast_slice(&[UpdateUniforms {
                delta: [delta, delta],
                pool_size: [pool_width, pool_length],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let normal_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Normal Uniforms"),
            contents: bytemuck::cast_slice(&[UpdateUniforms {
                delta: [delta, delta],
                pool_size: [pool_width, pool_length],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sphere_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sphere Volume Uniforms"),
            contents: bytemuck::cast_slice(&[SphereVolumeUniforms {
                old_center: [0.0; 4],
                new_center: [0.0; 4],
                radius: 0.25,
                strength: 0.04,
                pool_size: [pool_width, pool_length],
                shape_type: 0,
                _padding: [0.0; 3],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Water Sim Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create bind groups for ping-pong




        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Water Sim Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create pipelines
        let create_pipeline = |shader_source: &str, label: &str| {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
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
            })
        };

        let drop_pipeline = create_pipeline(DROP_SHADER, "Drop Pipeline");
        let update_pipeline = create_pipeline(UPDATE_SHADER, "Update Pipeline");
        let normal_pipeline = create_pipeline(NORMAL_SHADER, "Normal Pipeline");
        let sphere_pipeline = create_pipeline(SPHERE_VOLUME_SHADER, "Sphere Pipeline");

        Self {
            _texture_a: texture_a,
            texture_a_view,
            _texture_b: texture_b,
            texture_b_view,
            current: 0,
            sampler,
            drop_pipeline,
            update_pipeline,
            normal_pipeline,
            sphere_pipeline,
            drop_uniform_buffer,
            update_uniform_buffer,
            normal_uniform_buffer,
            sphere_uniform_buffer,
            bind_group_layout,
            pool_width,
            pool_length,
        }
    }

    /// Get the current water texture view (for reading)
    pub fn current_texture_view(&self) -> &wgpu::TextureView {
        if self.current == 0 {
            &self.texture_a_view
        } else {
            &self.texture_b_view
        }
    }

    /// Get the sampler
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    fn get_current_bind_group(&self, device: &wgpu::Device, uniform_buffer: &wgpu::Buffer) -> wgpu::BindGroup {


        let texture_view = if self.current == 0 {
            &self.texture_a_view
        } else {
            &self.texture_b_view
        };

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Water Sim Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn get_target_view(&self) -> &wgpu::TextureView {
        if self.current == 0 {
            &self.texture_b_view
        } else {
            &self.texture_a_view
        }
    }

    fn swap(&mut self) {
        self.current = 1 - self.current;
    }

    /// Add a drop/ripple at the given world coordinates
    pub fn add_drop(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        x: f32,
        z: f32,
        radius: f32,
        strength: f32,
    ) {
        // Normalize coordinates to -1..1
        let nx = x / (self.pool_width / 2.0);
        let nz = z / (self.pool_length / 2.0);

        let uniforms = DropUniforms {
            center: [nx, nz],
            radius,
            strength,
            pool_size: [self.pool_width, self.pool_length],
            _padding: [0.0, 0.0],
        };

        queue.write_buffer(&self.drop_uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let bind_group = self.get_current_bind_group(device, &self.drop_uniform_buffer);
        let target_view = self.get_target_view();

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Drop Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.drop_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..4, 0..1);
        }

        self.swap();
    }

    /// Update wave simulation (one step)
    pub fn step_simulation(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let delta = 1.0 / WATER_TEXTURE_SIZE as f32;
        let uniforms = UpdateUniforms {
            delta: [delta, delta],
            pool_size: [self.pool_width, self.pool_length],
        };

        queue.write_buffer(&self.update_uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let bind_group = self.get_current_bind_group(device, &self.update_uniform_buffer);
        let target_view = self.get_target_view();

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Update Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.update_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..4, 0..1);
        }

        self.swap();
    }

    /// Update normals
    pub fn update_normals(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let delta = 1.0 / WATER_TEXTURE_SIZE as f32;
        let uniforms = UpdateUniforms {
            delta: [delta, delta],
            pool_size: [self.pool_width, self.pool_length],
        };

        queue.write_buffer(&self.normal_uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let bind_group = self.get_current_bind_group(device, &self.normal_uniform_buffer);
        let target_view = self.get_target_view();

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Normal Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.normal_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..4, 0..1);
        }

        self.swap();
    }



    /// Move sphere through water (creates displacement)
    pub fn move_sphere(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        old_center: Vec3,
        new_center: Vec3,
        radius: f32,
        strength: f32,
        shape_type: i32,
    ) {
        // Normalize coordinates
        let scale_x = self.pool_width / 2.0;
        let scale_z = self.pool_length / 2.0;

        let uniforms = SphereVolumeUniforms {
            old_center: [old_center.x / scale_x, old_center.y, old_center.z / scale_z, 0.0],
            new_center: [new_center.x / scale_x, new_center.y, new_center.z / scale_z, 0.0],
            radius,
            strength,
            pool_size: [self.pool_width, self.pool_length],
            shape_type,
            _padding: [0.0; 3],
        };

        queue.write_buffer(&self.sphere_uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let bind_group = self.get_current_bind_group(device, &self.sphere_uniform_buffer);
        let target_view = self.get_target_view();

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sphere Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.sphere_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..4, 0..1);
        }

        self.swap();
    }

    /// Update pool dimensions
    pub fn update_dimensions(&mut self, width: f32, length: f32) {
        self.pool_width = width;
        self.pool_length = length;
    }
}
