//! Application module - main entry point and event loop handling

use std::sync::Arc;
use std::time::Instant;
use glam::Vec3;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

use crate::camera::Camera;
use crate::input::{InputManager, InteractionMode};
use crate::physics::PhysicsEngine;
use crate::renderer::Renderer;
use crate::water::Water;

use crate::ui::UiRenderer;
use crate::gui::{Gui, AppConfig as RunConfig, Shape, Texture, Backend, PoolShape};

#[derive(PartialEq)]
enum AppState {
    Configuring,
    Running,
}



/// Graphics state
struct GfxState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    water: Water,
    renderer: Renderer,

    ui: UiRenderer,
    gui: Gui,
}

impl GfxState {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find adapter");

        log::info!("Using adapter: {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let water = Water::new(&device);
        let renderer = Renderer::new(&device, &queue, surface_format, size.width, size.height);
        let ui = UiRenderer::new(&device, surface_format);
        let gui = Gui::new(window.clone(), &device, surface_format);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            water,
            renderer,
            ui,
            gui,
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.renderer.resize(&self.device, new_size.width, new_size.height);
        }
    }
}

/// Main application
pub struct Application {
    window: Option<Arc<Window>>,
    gfx: Option<GfxState>,
    camera: Camera,
    physics: PhysicsEngine,
    input: InputManager,
    last_frame: Instant,
    time: f32,
    paused: bool,
    frame_count: u32,
    accum_time: f32,
    state: AppState,
    run_config: RunConfig,
    current_fps: i32,
    dragged_object_index: Option<usize>,
    physics_accumulator: f32,
}


impl Application {
    pub async fn new(_event_loop: &EventLoop<()>) -> Self {
        Self {
            window: None,
            gfx: None,
            camera: Camera::default(),
            physics: PhysicsEngine::default(),
            input: InputManager::default(),
            last_frame: Instant::now(),
            time: 0.0,
            paused: false,
            frame_count: 0,
            accum_time: 0.0,
            current_fps: 60,
            dragged_object_index: None,
            state: AppState::Configuring,
            run_config: RunConfig::default(),
            physics_accumulator: 0.0,
        }
    }

    fn update(&mut self, dt: f32) {
        // Skip if time step is too large
        if dt > 0.5 {
            return;
        }

        self.time += dt;

        // FPS tracking
        self.frame_count += 1;
        self.accum_time += dt;
        if self.accum_time >= 1.0 {
            self.current_fps = self.frame_count as i32;
            self.frame_count = 0;
            self.accum_time -= 1.0;
        }

        // Fixed timestep for physics and water simulation
        const FIXED_DT: f32 = 1.0 / 60.0;
        self.physics_accumulator += dt;

        // Limit accumulator to prevent "spiral of death"
        if self.physics_accumulator > 0.25 {
            self.physics_accumulator = 0.25;
        }

        while self.physics_accumulator >= FIXED_DT {
            self.step_physics(FIXED_DT);
            self.physics_accumulator -= FIXED_DT;
        }
    }

    fn step_physics(&mut self, dt: f32) {
        let gfx = match &mut self.gfx {
            Some(g) => g,
            None => return,
        };

        if self.paused {
            return;
        }

        // Get water height at sphere position (simplified - using 0 for now)
        let water_height = 0.0;

        // Update physics
        self.physics.update(dt, water_height, self.input.mouse_point, self.dragged_object_index);

        // Process physics ripples
        if !self.physics.ripples.is_empty() {
             let mut encoder = gfx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Ripple Encoder"),
            });
            
            for ripple in &self.physics.ripples {
                 gfx.water.add_drop(
                    &gfx.device,
                    &gfx.queue,
                    &mut encoder,
                    ripple.x,
                    ripple.z,
                    ripple.radius,
                    ripple.strength,
                );
            }
            
            gfx.queue.submit(Some(encoder.finish()));
        }

        // Handle sphere dragging
        if let Some(delta) = self.input.get_sphere_drag_delta(self.camera.view_projection_matrix().inverse()) {
            if let Some(idx) = self.dragged_object_index {
                self.physics.move_by(idx, delta);
            }
        }

        // Handle camera orbit
        if self.input.mode == InteractionMode::OrbitCamera && self.input.mouse_pressed {
            let delta = self.input.get_orbit_delta();
            self.camera.orbit(delta.x, delta.y);
        }

        // Handle drop creation
        if let Some(pos) = self.input.get_drop_position() {
            let mut encoder = gfx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Drop Encoder"),
            });
            gfx.water.add_drop(
                &gfx.device,
                &gfx.queue,
                &mut encoder,
                pos.x,
                pos.z,
                0.03,
                0.02,
            );
            gfx.queue.submit(Some(encoder.finish()));
        }

        // Update water simulation
        let mut encoder = gfx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Sim Encoder"),
        });

        // Step simulation - with fixed DT we only need a small fixed number
        // Using 6 steps per fixed update - balanced for speed and performance
        for _ in 0..6 {
            gfx.water.step_simulation(&gfx.device, &gfx.queue, &mut encoder);
        }
        gfx.water.update_normals(&gfx.device, &gfx.queue, &mut encoder);

        // Sphere water interaction
        let shape_type = match self.run_config.shape {
            Shape::Sphere => 0,
            Shape::Torus => 1,
            Shape::Tetrahedron => 2,
            Shape::Cube => 3,
        };

        gfx.water.move_objects(
            &gfx.device,
            &gfx.queue,
            &mut encoder,
            &self.physics.objects,
            self.physics.impact_strength,
            self.physics.radius,
            shape_type,
        );

        gfx.queue.submit(Some(encoder.finish()));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let gfx = match &mut self.gfx {
            Some(g) => g,
            None => return Ok(()),
        };

        // If configuring, draw GUI only (or overlay)
        if self.state == AppState::Configuring {
             let output = gfx.surface.get_current_texture()?;
             let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
             
             let mut encoder = gfx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GUI Encoder"),
            });
            
            let window = self.window.as_ref().unwrap();
            let mut run_clicked = false;
            let config = &mut self.run_config;

            gfx.gui.render(&gfx.device, &gfx.queue, window, &mut encoder, &view, |ctx| {
                egui::Window::new("Run Parameters")
                    .resizable(false)
                    .collapsible(false)
                    .show(ctx, |ui| {
                        ui.set_min_width(550.0);
                        ui.vertical_centered(|ui| {
                            ui.heading("Run Parameters");
                        });
                        ui.add_space(8.0);
                         
                        ui.horizontal(|ui| {
                            ui.label("Gravity (g)");
                            ui.add_space(10.0);
                            ui.add(egui::Slider::new(&mut config.gravity, 0.25..=5.0).step_by(0.25).text("step 0.25"));
                        });
                         
                        ui.add_space(16.0);
                         
                        // 3-column layout: Shape, Textures, Pool Shape
                        ui.columns(3, |columns| {
                            columns[0].vertical(|ui| {
                                ui.label("Object Shape:");
                                ui.radio_value(&mut config.shape, Shape::Sphere, "Sphere");
                                ui.radio_value(&mut config.shape, Shape::Torus, "Torus");
                                ui.radio_value(&mut config.shape, Shape::Tetrahedron, "Tetrahedron");
                                ui.radio_value(&mut config.shape, Shape::Cube, "Cube");
                            });
                             
                            columns[1].vertical(|ui| {
                                ui.label("Textures:");
                                ui.radio_value(&mut config.texture, Texture::Glass, "Glass");
                                ui.radio_value(&mut config.texture, Texture::Wood, "Wood");
                                ui.radio_value(&mut config.texture, Texture::Steel, "Steel");
                                ui.radio_value(&mut config.texture, Texture::Ice, "Ice");
                            });
                             
                            columns[2].vertical(|ui| {
                                ui.label("Pool Shape:");
                                ui.radio_value(&mut config.pool_shape, PoolShape::Tube, "Tube");
                                ui.radio_value(&mut config.pool_shape, PoolShape::Cube, "Cube");
                                ui.radio_value(&mut config.pool_shape, PoolShape::Cuboid, "Cuboid");
                                ui.radio_value(&mut config.pool_shape, PoolShape::Frustum, "Frustum");
                            });
                        });
                         
                        ui.add_space(16.0);
                        
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label("Object Count:");
                                ui.add(egui::Slider::new(&mut config.object_count, 1..=50));
                            });
                            
                            ui.add_space(20.0);

                            // Backend selection
                            ui.vertical(|ui| {
                                ui.label("Graphics Backend:");
                                ui.horizontal(|ui| {
                                    ui.radio_value(&mut config.backend, Backend::OpenGL, "OpenGL");
                                    ui.radio_value(&mut config.backend, Backend::Vulkan, "Vulkan");
                                });
                                ui.horizontal(|ui| {
                                    ui.radio_value(&mut config.backend, Backend::Dx11, "DX11");
                                    ui.radio_value(&mut config.backend, Backend::Dx12, "DX12");
                                    ui.radio_value(&mut config.backend, Backend::Auto, "Auto");
                                });
                            });
                        });

                        ui.add_space(10.0);
                        
                        ui.horizontal(|ui| {
                           ui.vertical(|ui| {
                               ui.label("Light Color:");
                               let mut color_srgba = egui::Color32::from_rgb(config.light_color[0], config.light_color[1], config.light_color[2]);
                               ui.color_edit_button_srgba(&mut color_srgba);
                               config.light_color = [color_srgba.r(), color_srgba.g(), color_srgba.b()];
                               ui.label("color wheel");
                               
                               ui.add_space(10.0);
                               ui.label("Intensity:");
                               ui.add(egui::Slider::new(&mut config.light_intensity, 0.0..=5.0));
                           });
                           
                           ui.vertical(|ui| {
                               ui.horizontal(|ui| {
                                   ui.label("Red");
                                   ui.add(egui::Slider::new(&mut config.light_color[0], 0..=255));
                               });
                               ui.horizontal(|ui| {
                                   ui.label("Green");
                                   ui.add(egui::Slider::new(&mut config.light_color[1], 0..=255));
                               });
                               ui.horizontal(|ui| {
                                   ui.label("Blue");
                                   ui.add(egui::Slider::new(&mut config.light_color[2], 0..=255));
                               });
                           });
                        });
                        
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut config.enable_gi, "Enable Global Illumination");
                            ui.add_space(20.0);
                            ui.checkbox(&mut config.enable_raytracing, "Enable Ray Tracing");
                        });
                         
                        ui.add_space(24.0);
                        ui.horizontal(|ui| {
                            if ui.button("Run Simulation").clicked() {
                                run_clicked = true;
                            }
                        });
                    });
            });
            
            gfx.queue.submit(Some(encoder.finish()));
            output.present();
            
            if run_clicked {
                self.state = AppState::Running;
                
                {
                    let config = &self.run_config;
                    self.physics.gravity = Vec3::new(0.0, -9.81 * config.gravity, 0.0);
                    self.physics.pool_shape = config.pool_shape;
                    
                    // Set material density based on texture
                    self.physics.material_density = config.texture.density();
                    
                    let (width, length) = match config.pool_shape {
                        PoolShape::Cuboid => (2.0, 3.0),
                        _ => (2.0, 2.0),
                    };
                    
                    self.physics.pool_width = width;
                    self.physics.pool_length = length;
                    gfx.renderer.pool_width = width;
                    gfx.renderer.pool_length = length;
                    gfx.water.pool_width = width;
                    gfx.water.pool_length = length;
                    self.physics.wall_height = gfx.renderer.wall_height;
                    
                    self.physics.reset_objects(config.object_count);
                    
                    let shape_name = match config.shape {
                        Shape::Sphere => "Sphere",
                        Shape::Torus => "Torus",
                        Shape::Tetrahedron => "Tetrahedron",
                        Shape::Cube => "Cube",
                    };
                    gfx.renderer.update_object_mesh(&gfx.device, shape_name);
                    self.physics.set_shape(shape_name);
                    
                    let pool_shape_name = match config.pool_shape {
                        PoolShape::Cube => "Cube",
                        PoolShape::Cuboid => "Cuboid",
                        PoolShape::Frustum => "Frustum",
                        PoolShape::Tube => "Tube",
                    };
                    gfx.renderer.update_pool_mesh(&gfx.device, pool_shape_name);
                    
                    let c = config.light_color;
                     gfx.renderer.common_uniform.light_color = [
                        c[0] as f32 / 255.0,
                        c[1] as f32 / 255.0,
                        c[2] as f32 / 255.0,
                        1.0
                    ];
                    gfx.renderer.common_uniform.enable_gi = if config.enable_gi { 1 } else { 0 };
                    gfx.renderer.common_uniform.enable_raytracing = if config.enable_raytracing { 1 } else { 0 };
                }

                // Initial Drops Logic Inline
                {
                    let mut encoder = gfx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Init Drops Encoder"),
                    });
            
                    for i in 0..20 {
                        let x = (rand_float() * 2.0 - 1.0) * 0.8;
                        let z = (rand_float() * 2.0 - 1.0) * 0.8;
                        let strength = if i % 2 == 0 { 0.01 } else { -0.01 };
                        gfx.water.add_drop(&gfx.device, &gfx.queue, &mut encoder, x, z, 0.03, strength);
                    }
            
                    gfx.queue.submit(Some(encoder.finish()));
                }
            }
            
            return Ok(());
        }

        let output = gfx.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Update uniforms
        let shape_type = match self.run_config.shape {
            Shape::Sphere => 0,
            Shape::Torus => 1,
            Shape::Tetrahedron => 2,
            Shape::Cube => 3,
        };
        
        let texture_type = match self.run_config.texture {
            Texture::Glass => 0,
            Texture::Wood => 1,
            Texture::Steel => 2,
            Texture::Ice => 3,
        };

        let lc = self.run_config.light_color;
        let intensity = self.run_config.light_intensity;
        
        let pool_shape_idx = match self.run_config.pool_shape {
            PoolShape::Cube | PoolShape::Cuboid => 0,
            PoolShape::Frustum => 1,
            PoolShape::Tube => 2,
        };

        gfx.renderer.update_uniforms(
            &gfx.queue,
            &self.camera,
            &self.physics.objects,
            self.physics.radius,
            self.time,
            shape_type,
            texture_type,
            [
                (lc[0] as f32 / 255.0) * intensity,
                (lc[1] as f32 / 255.0) * intensity,
                (lc[2] as f32 / 255.0) * intensity
            ],
            pool_shape_idx,
            self.run_config.enable_gi,
            self.run_config.enable_raytracing,
        );

        // Update FPS UI
        gfx.ui.update(
            &gfx.queue, 
            self.current_fps, 
            self.physics.gravity_enabled, 
            self.physics.mouse_repulsion_enabled, 
            self.paused
        );

        let mut encoder = gfx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Render caustics
        gfx.renderer.render_caustics(&gfx.device, &mut encoder, &gfx.water);

        // Render main scene
        gfx.renderer.render(&gfx.device, &mut encoder, &view, &gfx.water);

        // Render UI
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            gfx.ui.render(&mut pass);
        }

        gfx.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }



    fn on_resize(&mut self, width: u32, height: u32) {
        self.camera.set_aspect(width as f32, height as f32);
        self.input.set_screen_size(width as f32, height as f32);

        // Keep pool dimensions fixed - don't auto-resize on window resize
        // Pool dimensions are set on startup and by update_pool_dimensions
    }
}

// Simple random number generator (for initial drops)
fn rand_float() -> f32 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u32;
    (seed.wrapping_mul(1103515245).wrapping_add(12345) % 1000) as f32 / 1000.0
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Rust GL Water - Poolcore Demo")
            .with_inner_size(PhysicalSize::new(600, 600));

        let window = Arc::new(event_loop.create_window(window_attrs).expect("Failed to create window"));
        
        let gfx = pollster::block_on(GfxState::new(window.clone()));
        
        self.window = Some(window.clone());
        self.gfx = Some(gfx);
        
        let size = window.inner_size();
        self.on_resize(size.width, size.height);
        self.last_frame = Instant::now();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // Handle GUI events first
        if self.state == AppState::Configuring {
            if let Some(gfx) = &mut self.gfx {
                if gfx.gui.handle_event(self.window.as_ref().unwrap(), &event) {
                    // If GUI consumed the event, skip game logic unless it's a resize/close
                    if !matches!(event, WindowEvent::Resized(..) | WindowEvent::CloseRequested) {
                        return;
                    }
                }
            }
        }
        
        match event {
            WindowEvent::CloseRequested => {
                log::info!("Close requested, exiting...");
                event_loop.exit();
            }
            
            WindowEvent::Resized(physical_size) => {
                if let Some(gfx) = &mut self.gfx {
                    gfx.resize(physical_size);
                    self.on_resize(physical_size.width, physical_size.height);
                }
            }
            
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32();
                self.last_frame = now;
                
                self.update(dt);
                
                match self.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        if let Some(gfx) = &mut self.gfx {
                            gfx.resize(gfx.size);
                        }
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        log::error!("Out of memory!");
                        event_loop.exit();
                    }
                    Err(e) => {
                        log::warn!("Surface error: {:?}", e);
                    }
                }
                
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            
            WindowEvent::CursorMoved { position, .. } => {
                // Skip input if configuring
                if self.state == AppState::Configuring {
                    return;
                }
                if self.gfx.is_some() {
                    let view_proj_inv = self.camera.view_projection_matrix().inverse();
                    self.input.on_mouse_move(
                        position.x as f32,
                        position.y as f32,
                        view_proj_inv,
                    );
                }
            }
            
            WindowEvent::MouseInput { state, button, .. } => {
                // Skip input if configuring
                if self.state == AppState::Configuring {
                    return;
                }
                match (state, button) {
                    (ElementState::Pressed, MouseButton::Left) => {
                        let view_proj_inv = self.camera.view_projection_matrix().inverse();
                        let view_dir = self.camera.target - self.camera.position();
                        
                        let hit_idx = self.input.on_mouse_down(
                            self.input.mouse_pos.x,
                            self.input.mouse_pos.y,
                            view_proj_inv,
                            view_dir.normalize(),
                            &self.physics.objects,
                            self.physics.radius,
                            self.physics.pool_width,
                            self.physics.pool_length,
                            self.gfx.as_ref().unwrap().renderer.pool_height,
                            self.gfx.as_ref().unwrap().renderer.wall_height,
                            &self.run_config.pool_shape,
                        );
                        self.dragged_object_index = hit_idx;
                    }
                    (ElementState::Released, MouseButton::Left) => {
                        self.input.on_mouse_up();
                        self.dragged_object_index = None;
                    }
                    _ => {}
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if self.state == AppState::Configuring {
                    return;
                }
                let delta = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y * 0.5,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
                };
                self.camera.zoom(delta);
            }
            
            WindowEvent::KeyboardInput { event, .. } => {
                // in configuring, still allow allow esc to exit
                if self.state == AppState::Configuring {
                    if event.state == ElementState::Pressed {
                         if let winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) = event.logical_key {
                             event_loop.exit();
                         }
                    }
                    return;
                }

                if event.state == ElementState::Pressed {
                    use winit::keyboard::{Key, NamedKey};
                    match event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            event_loop.exit();
                        }
                        Key::Named(NamedKey::Space) => {
                             self.paused = !self.paused;
                        }
                        Key::Character(s) => {
                            match s.as_str() {
                                "l" | "L" => {
                                    // Update light direction to camera view
                                    if let Some(gfx) = &mut self.gfx {
                                        let dir = self.camera.target - self.camera.position();
                                        gfx.renderer.light_dir = dir.normalize();
                                    }
                                }
                                "k" | "K" => {
                                    // Toggle mouse repulsion (makes it easier to grab objects)
                                    self.physics.mouse_repulsion_enabled = !self.physics.mouse_repulsion_enabled;
                                    log::info!("Mouse repulsion: {}", if self.physics.mouse_repulsion_enabled { "ON" } else { "OFF" });
                                }
                                "g" | "G" => {
                                    self.physics.gravity_enabled = !self.physics.gravity_enabled;
                                }
                                "p" | "P" => {
                                    if let Some(gfx) = &mut self.gfx {
                                        gfx.ui.show_ui = !gfx.ui.show_ui;
                                    }
                                }
                                "o" | "O" => {
                                    self.state = AppState::Configuring;
                                    // Make sure cursor is visible
                                    self.window.as_ref().unwrap().set_cursor_visible(true); 
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
            
            _ => {}
        }
    }
}
